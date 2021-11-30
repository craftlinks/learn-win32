use std::{ffi::c_void, mem};
use windows::{
    core::{Interface, Result},
    Foundation::Numerics::Matrix3x2,
    Win32::{
        Foundation::{
            CloseHandle, BOOL, D2DERR_RECREATE_TARGET, HANDLE, HWND, LPARAM,
            LRESULT, PSTR, RECT, SYSTEMTIME, WPARAM,
        },
        Graphics::{
            Direct2D::{
                Common::{
                    D2D1_ALPHA_MODE_UNKNOWN, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
                    D2D_POINT_2F, D2D_SIZE_U,
                },
                D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget,
                ID2D1SolidColorBrush, D2D1_BRUSH_PROPERTIES,
                D2D1_DEBUG_LEVEL_INFORMATION, D2D1_ELLIPSE,
                D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED,
                D2D1_FEATURE_LEVEL_DEFAULT, D2D1_HWND_RENDER_TARGET_PROPERTIES,
                D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
                D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
            },
            Dxgi::Common::DXGI_FORMAT_UNKNOWN,
            Gdi::{
                BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT,
            },
        },
        System::{
            Com::{
                CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED,
                COINIT_DISABLE_OLE1DDE,
            },
            LibraryLoader::GetModuleHandleW,
            SystemInformation::GetLocalTime,
            Threading::{SetWaitableTimer, WAIT_OBJECT_0},
            WindowsProgramming::{CreateWaitableTimerA, INFINITE},
        },
        UI::WindowsAndMessaging::{
            CreateWindowExA, DefWindowProcA, DispatchMessageA, GetClientRect,
            GetWindowLongPtrA, LoadCursorW, MsgWaitForMultipleObjects,
            PeekMessageA, PostQuitMessage, RegisterClassA, SetWindowLongPtrA,
            TranslateMessage, CREATESTRUCTA, CS_HREDRAW, CS_OWNDC, CS_VREDRAW,
            CW_USEDEFAULT, GWLP_USERDATA, IDC_CROSS, MSG, PM_REMOVE,
            QS_ALLINPUT, WM_CREATE, WM_DESTROY, WM_DISPLAYCHANGE, WM_PAINT,
            WM_QUIT, WM_SIZE, WNDCLASSA, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
};

pub struct Scene {
    factory: ID2D1Factory,
    render_target: Option<ID2D1HwndRenderTarget>,
    fill_brush: Option<ID2D1SolidColorBrush>,
    stroke_brush: Option<ID2D1SolidColorBrush>,
    ellipse: D2D1_ELLIPSE,
    tick: (D2D_POINT_2F, D2D_POINT_2F),
}

impl Scene {
    pub fn new() -> Result<Scene> {
        let factory = create_factory()?;

        Ok(Scene {
            factory,
            render_target: None,
            fill_brush: None,
            stroke_brush: None,
            ellipse: D2D1_ELLIPSE::default(),
            tick: (D2D_POINT_2F::default(), D2D_POINT_2F::default()),
        })
    }

    fn create_graphics_resources(
        &mut self,
        window_handle: &HWND,
    ) -> Option<()> {
        let mut rect: RECT = RECT::default();
        let mut status = Some(());
        if self.render_target.is_none() {
            unsafe {
                if window_handle.0 != 0 {
                    GetClientRect(window_handle, &mut rect)
                        .expect("Problem obtaining client RECT area.");
                } else {
                    status = None;
                }

                let size = D2D_SIZE_U {
                    width: (rect.right - rect.left) as u32,
                    height: (rect.bottom - rect.top) as u32,
                };

                if let Some(render_target) =
                    self.create_render_target(size, &window_handle)
                {
                    self.render_target = Some(render_target);
                    status = self.create_device_dependent_resources().ok();
                }

                self.calculate_layout();
            }
        }
        status
    }

    fn create_render_target(
        &self,
        pixel_size: D2D_SIZE_U,
        window_handle: &HWND,
    ) -> Option<ID2D1HwndRenderTarget> {
        unsafe {
            let render_properties = {
                let pixel_format = D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_UNKNOWN,
                    alphaMode: D2D1_ALPHA_MODE_UNKNOWN,
                };

                D2D1_RENDER_TARGET_PROPERTIES {
                    r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                    pixelFormat: pixel_format,
                    dpiX: 0.0,
                    dpiY: 0.0,
                    usage: D2D1_RENDER_TARGET_USAGE_NONE,
                    minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
                }
            };

            let hwnd_render_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: *window_handle,
                pixelSize: pixel_size,
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };

            let target = self
                .factory
                .CreateHwndRenderTarget(
                    &render_properties,
                    &hwnd_render_properties,
                )
                .unwrap();
            Some(target)
        }
    }

    fn create_device_dependent_resources(&mut self) -> Result<()> {
        let fill_color: D2D1_COLOR_F = D2D1_COLOR_F {
            r: 1.0_f32,
            g: 1.0_f32,
            b: 0.0_f32,
            a: 1.0_f32,
        };

        let stroke_color: D2D1_COLOR_F = D2D1_COLOR_F {
            r: 0.0_f32,
            g: 0.0_f32,
            b: 0.0_f32,
            a: 1.0_f32,
        };

        let brush_props: D2D1_BRUSH_PROPERTIES = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0_f32,
            transform: Matrix3x2::identity(),
        };

        unsafe {
            let fill_brush = self
                .render_target
                .as_ref()
                .unwrap()
                .CreateSolidColorBrush(&fill_color, &brush_props)
                .expect("Failed to create Solid Color Brush.");
            self.fill_brush = Some(fill_brush);

            let stroke_brush = self
                .render_target
                .as_ref()
                .unwrap()
                .CreateSolidColorBrush(&stroke_color, &brush_props)
                .expect("Failed to create Solid Color Brush.");
            self.stroke_brush = Some(stroke_brush);
        }
        Ok(())
    }

    fn discard_device_dependent_resources(&mut self) {
        self.fill_brush = None;
        self.stroke_brush = None;
    }

    fn calculate_layout(&mut self) {
        if let Some(render_target) = &self.render_target {
            unsafe {
                let size = render_target.GetSize();
                let x = size.width / 2_f32;
                let y = size.height / 2_f32;
                let radius = x.min(y);
                self.ellipse = D2D1_ELLIPSE {
                    point: D2D_POINT_2F { x, y },
                    radiusX: radius,
                    radiusY: radius,
                };

                // Calculate tick marks. Worst case we will have to skip this

                let pt1: D2D_POINT_2F = D2D_POINT_2F {
                    x: self.ellipse.point.x,
                    y: self.ellipse.point.y - (self.ellipse.radiusY * 0.9_f32),
                };

                let pt2: D2D_POINT_2F = D2D_POINT_2F {
                    x: self.ellipse.point.x,
                    y: self.ellipse.point.y - (self.ellipse.radiusY),
                };

                self.tick = (pt1, pt2);

                // This part will have to be drawm during the render function...
                // as it seems I don't have the TransformPoint method available
                // for i in 0..11 {
                //     let mat: Matrix3x2 = Matrix3x2::rotation(
                //         (360.0_f32 / 12.0_f32) * i as f32,
                //          self.ellipse.point.x, self.ellipse.point.y
                //     );

                //     self.ticks[i*2] = mat.;
                //     self.ticks[i*2 + 1] = mat.TransformPoint(pt2);
                // }
            }
        }
    }

    fn render(&mut self, window_handle: &HWND) {
        self.create_graphics_resources(&window_handle)
            .expect("Failed creating graphics resources.");
        assert!(self.render_target.is_some());

        // Initiates Direct2D drawing on this render target.
        unsafe { self.render_target.as_ref().unwrap().BeginDraw() };

        self.render_scene();

        unsafe {
            if let Err(error) = self
                .render_target
                .as_ref()
                .unwrap()
                .EndDraw(std::ptr::null_mut(), std::ptr::null_mut())
            {
                if error.code() == D2DERR_RECREATE_TARGET {
                    self.discard_device_dependent_resources();
                    self.render_target = None;
                }
            }
        };
    }

    fn render_scene(&self) {
        unsafe {
            let render_target = self.render_target.as_ref().unwrap();
            render_target.Clear(&D2D1_COLOR_F {
                r: 135_f32 / 256_f32,
                g: 206_f32 / 256_f32,
                b: 235_f32 / 256_f32,
                a: 0.8_f32,
            });
            render_target
                .FillEllipse(&self.ellipse, self.fill_brush.as_ref().unwrap());
            render_target.DrawEllipse(
                &self.ellipse,
                self.stroke_brush.as_ref().unwrap(),
                1.0_f32,
                None,
            );

            // TODO Geert: Draw the tick marks in a loop

            // Draw hands
            let mut time: SYSTEMTIME = SYSTEMTIME::default();
            GetLocalTime(&mut time);

            // 60 minutes = 30 degrees, 1 minute = 0.5 degree
            let hour_angle = (360.0_f32 / 12.0_f32) * (time.wHour as f32)
                + (time.wMinute as f32 * 0.5_f32);
            let minute_angle = (360.0_f32 / 60_f32) * (time.wMinute as f32);
            let second_angle = (360.0_f32 / 60_f32) * (time.wSecond as f32)
                + (360.0_f32 / 60000_f32) * (time.wMilliseconds as f32);

            self.draw_clock_hand(0.6_f32, hour_angle, 6.0);
            self.draw_clock_hand(0.85_f32, minute_angle, 4.0);
            self.draw_clock_hand(0.85_f32, second_angle, 1.0);

            // Restore the identity transformation.
            render_target.SetTransform(&Matrix3x2::identity());
        }
    }

    fn draw_clock_hand(&self, hand_length: f32, angle: f32, stroke_width: f32) {
        let render_target = self.render_target.as_ref().unwrap();
        unsafe {
            render_target.SetTransform(&Matrix3x2::rotation(
                angle,
                self.ellipse.point.x,
                self.ellipse.point.y,
            ));

            // endPoint defines one end of the hand.
            let end_point: D2D_POINT_2F = D2D_POINT_2F {
                x: self.ellipse.point.x,
                y: self.ellipse.point.y - (self.ellipse.radiusY * hand_length),
            };

            // Draw a line from the center of the ellipse to endPoint.
            render_target.DrawLine(
                self.ellipse.point,
                end_point,
                self.stroke_brush.as_ref().unwrap(),
                stroke_width,
                None,
            );
        }
    }

    fn resize(&mut self, window_handle: HWND) {
        if self.render_target.is_some() {
            let mut rc: RECT = RECT::default();
            unsafe { GetClientRect(window_handle, &mut rc) };

            let size: D2D_SIZE_U = D2D_SIZE_U {
                width: (rc.right - rc.left) as u32,
                height: (rc.bottom - rc.top) as u32,
            };

            unsafe {
                self.render_target
                    .as_ref()
                    .unwrap()
                    .Resize(&size)
                    .expect("Failed at Resizing the window.")
            };
            self.calculate_layout();
            unsafe {
                // The InvalidateRect function forces a repaint by adding the
                // entire client area to the window's update region.
                InvalidateRect(window_handle, std::ptr::null_mut(), BOOL(0))
            };
        }
    }

    fn cleanup(&mut self) {
        self.discard_device_dependent_resources();
    }
}

pub struct WindowContext {
    timer_handle: Option<HANDLE>,
    window_handle: Option<HWND>,
    scene: Option<Scene>,
}

impl WindowContext {
    pub fn new() -> Self {
        WindowContext {
            window_handle: None,
            scene: None,
            timer_handle: None,
        }
    }

    fn initialize_timer(&mut self) -> Option<()> {
        unsafe {
            // Creates or opens a waitable timer object and returns a handle to
            // the object.
            self.timer_handle = Some(CreateWaitableTimerA(
                std::ptr::null(),
                BOOL(0),
                PSTR("\0".as_ptr() as *mut u8),
            ));
            if self.timer_handle.unwrap().0 == 0 {
                return None;
            }
            let mut due_time: i64 = 0;

            // Activates the specified waitable timer. When the due time arrives
            // (1s/60), the timer is signaled and the thread that set the timer
            // calls the optional completion routine.
            if !SetWaitableTimer(
                self.timer_handle,
                &mut due_time,
                1000 / 60,
                None,
                std::ptr::null(),
                BOOL(0),
            )
            .as_bool()
            {
                // Closes an the timer object handle.
                CloseHandle(self.timer_handle.unwrap());
                self.timer_handle = None;
                return None;
            }
            return Some(());
        }
    }

    fn wait_timer(&self) {
        // Wait until the timer expires or any message is posted.
        unsafe {
            if MsgWaitForMultipleObjects(
                1,
                &self.timer_handle.unwrap(),
                BOOL(0),
                INFINITE,
                QS_ALLINPUT,
            ) == WAIT_OBJECT_0
            {
                InvalidateRect(
                    self.window_handle,
                    std::ptr::null_mut(),
                    BOOL(0),
                );
            }
        }
    }
}

fn create_factory() -> Result<ID2D1Factory> {
    let mut options = D2D1_FACTORY_OPTIONS::default();

    if cfg!(debug_assertions) {
        options.debugLevel = D2D1_DEBUG_LEVEL_INFORMATION;
    }

    let mut result = None;
    unsafe {
        D2D1CreateFactory(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            &ID2D1Factory::IID,
            &options,
            std::mem::transmute(&mut result),
        )
        .map(|()| result.unwrap())
    }
}
fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(
            std::ptr::null_mut(),
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        )
        .expect("Failed to initialize COM");

        let instance = GetModuleHandleW(None);
        debug_assert!(instance.0 != 0);

        let window_class = b"window\0";
        let window_context: *const WindowContext = &WindowContext::new();

        let wc = WNDCLASSA {
            hInstance: instance,
            lpszClassName: PSTR(window_class.as_ptr() as *mut u8),
            lpfnWndProc: Some(wndproc),
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
            hCursor: LoadCursorW(None, IDC_CROSS),

            ..Default::default()
        };

        let atom = RegisterClassA(&wc);
        debug_assert!(atom != 0);

        let handle = CreateWindowExA(
            Default::default(),
            PSTR(window_class.as_ptr() as _),
            PSTR(b"This is a sample window\0".as_ptr() as *mut u8),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            wc.hInstance,
            window_context as *const c_void,
        );
        debug_assert!(handle.0 != 0);

        let mut message = MSG::default();
        while message.message != WM_QUIT {
            if PeekMessageA(&mut message, HWND(0), 0, 0, PM_REMOVE).into() {
                TranslateMessage(&mut message);
                DispatchMessageA(&mut message);
            }
            window_context
                .as_ref()
                .expect("oops, window was not initialized!")
                .wait_timer();
        }
        println!("end of program");
        CoUninitialize();
        Ok(())
    }
}

extern "system" fn wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_CREATE {
            let createstruct = &mut *(lparam.0 as *mut CREATESTRUCTA);
            let init_context: &mut WindowContext =
                mem::transmute(createstruct.lpCreateParams);
            init_context.window_handle = Some(hwnd);
            init_context.initialize_timer();
            init_context.scene = Scene::new().ok();

            SetWindowLongPtrA(
                hwnd,
                GWLP_USERDATA,
                mem::transmute(init_context),
            );
        }

        let window_context: &mut WindowContext = {
            let user_data_ = GetWindowLongPtrA(hwnd, GWLP_USERDATA);
            mem::transmute(user_data_)
        };

        match message as u32 {
            WM_PAINT | WM_DISPLAYCHANGE => {
                println!("WM_PAINT");
                let mut ps = PAINTSTRUCT {
                    ..Default::default()
                };
                BeginPaint(window_context.window_handle.unwrap(), &mut ps);

                let window_handle = window_context
                    .window_handle
                    .expect("No valid window handle.");
                let scene = window_context.scene.as_mut().unwrap();
                scene.render(&window_handle);

                EndPaint(window_handle, &mut ps);
                LRESULT(0)
            }

            WM_SIZE => {
                let window_handle = window_context
                    .window_handle
                    .expect("No valid window handle.");
                let scene = window_context.scene.as_mut().unwrap();
                scene.resize(window_handle);
                InvalidateRect(window_handle, std::ptr::null(), BOOL(0));
                LRESULT(0)
            }

            WM_DESTROY => {
                println!("WM_DESTROY");
                if !CloseHandle(window_context.timer_handle.unwrap()).as_bool()
                {
                    println!("failed to destro the timer");
                    panic!()
                }

                window_context.scene.as_mut().unwrap().cleanup();

                PostQuitMessage(0);
                println!("post quit message");
                LRESULT(0)
            }
            _ => DefWindowProcA(hwnd, message, wparam, lparam),
        }
    }
}
