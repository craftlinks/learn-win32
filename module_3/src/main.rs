use std::{ffi::c_void, mem};

use windows::{
    core::Interface,
    Foundation::Numerics::Matrix3x2,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, PSTR, RECT, WPARAM, BOOL},
        Graphics::{
            Direct2D::{
                Common::{
                    D2D1_ALPHA_MODE_UNKNOWN,
                    D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_SIZE_U, D2D_POINT_2F,
                },
                D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget,
                ID2D1SolidColorBrush, D2D1_BRUSH_PROPERTIES,
                D2D1_DEBUG_LEVEL_INFORMATION, D2D1_ELLIPSE,
                D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED,
                D2D1_FEATURE_LEVEL_DEFAULT, D2D1_HWND_RENDER_TARGET_PROPERTIES,
                D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
                D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
            },
            Dxgi::Common::{DXGI_FORMAT_UNKNOWN},
            Gdi::{BeginPaint, EndPaint, PAINTSTRUCT, InvalidateRect},
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExA, DefWindowProcA, DispatchMessageA, GetClientRect,
            GetMessageA, GetWindowLongPtrA, LoadCursorW, PostQuitMessage,
            RegisterClassA, SetWindowLongPtrA, CREATESTRUCTA,
            CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA,
            IDC_CROSS, MSG, WM_CREATE, WM_SIZE, WM_DESTROY, WM_PAINT, WNDCLASSA,
            WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
    },
};

use windows::core::Result;

// type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct WindowContext {
    window_handle: Option<HWND>,

    // Creates Direct2D resources.
    factory: ID2D1Factory,

    // Renders drawing instructions to a window.
    render_target: Option<ID2D1HwndRenderTarget>,

    // Paints an area with a solid color.
    brush: Option<ID2D1SolidColorBrush>,

    // Contains the center point, x-radius, and y-radius of an ellipse.
    ellipse: D2D1_ELLIPSE,
}

impl WindowContext {
    pub fn new() -> Result<Self> {
        let window_handle = None;

        let factory = create_factory()?;

        // Needs to be obtained once a window handle has been created
        let render_target = None;
        let brush = None;

        let ellipse = D2D1_ELLIPSE::default();

        Ok(WindowContext {
            window_handle,
            factory,
            render_target,
            brush,
            ellipse,
        })
    }

    pub fn calculate_layout(&mut self) {
        if let Some(render_target) = &self.render_target {
            unsafe {
                let size = render_target.GetSize();
                let x = size.width/2_f32;
                let y = size.height/2_f32;
                let radius = x.min(y);
                self.ellipse = D2D1_ELLIPSE{
                    point: D2D_POINT_2F{ x, y },
                    radiusX: radius,
                    radiusY: radius,
                };
            }
        }
    }

    fn create_graphics_resources(&mut self) -> Result<()> {
        let mut rect: RECT = RECT::default();

        if self.render_target.is_none() {
            unsafe {
                GetClientRect(self.window_handle, &mut rect)
                    .expect("Problem obtaining client RECT area.");

                let size = D2D_SIZE_U {
                    width: (rect.right - rect.left) as u32,
                    height: (rect.bottom - rect.top) as u32,
                };

                self.render_target = self.create_render_target(size);

                if self.render_target.is_some() {
                    let color: D2D1_COLOR_F = D2D1_COLOR_F {
                        r: 0.2_f32,
                        g: 0.5_f32,
                        b: 0.2_f32,
                        a: 1.0_f32,
                    };
                    let brush_props: D2D1_BRUSH_PROPERTIES =
                        D2D1_BRUSH_PROPERTIES {
                            opacity: 1.0_f32,
                            transform: Matrix3x2::identity(),
                        };
                    let brush = self
                        .render_target
                        .as_ref()
                        .unwrap()
                        .CreateSolidColorBrush(&color, &brush_props)
                        .expect("Failed to create Solid Color Brush.");
                    self.brush = Some(brush);

                    self.calculate_layout();
                }
            }
        }
        Ok(())
    }

    fn create_render_target(
        &self,
        pixel_size: D2D_SIZE_U,
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
                hwnd: self
                    .window_handle
                    .expect("Current window handle was None."),
                pixelSize: pixel_size,
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };

            // Creates an ID2D1HwndRenderTarget, a render target that renders to
            // a window.
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

    pub fn discard_graphics_resources(&mut self) {
        self.render_target = None;
        self.brush = None;

    }

    pub fn on_paint(&mut self) {
        self.create_graphics_resources().expect("Failed creating graphics resources.");

        let mut ps = PAINTSTRUCT {
            ..Default::default()
        };
        unsafe {
            let hdc = BeginPaint(self.window_handle, &mut ps);
            debug_assert!(hdc.0 != 0);
            
            // Initiates Direct2D drawing on this render target.
            self.render_target.as_ref().unwrap().BeginDraw();

            // Clears the drawing area to the specified color.
            self.render_target.as_ref().unwrap().Clear( &D2D1_COLOR_F{r: 135_f32/256_f32, g:  206_f32/256_f32, b: 235_f32/256_f32,a: 0.8_f32});

            // Paints the interior of the specified ellipse.
            self.render_target.as_ref().unwrap().FillEllipse(&self.ellipse, self.brush.as_ref().unwrap());

            // Ends drawing operations on the render target and indicates the
            // current error state and associated tags.
            self.render_target.as_ref()
                .unwrap()
                .EndDraw(std::ptr::null_mut(), std::ptr::null_mut())
                .map_err(|_| {
                self.discard_graphics_resources();
            }).unwrap();
        
            EndPaint(self.window_handle, &ps);
        }
    }

    pub fn resize(&mut self) {
        if self.render_target.is_some() {
            let mut rc: RECT = RECT::default();
            unsafe {GetClientRect(self.window_handle, &mut rc)};

            let size: D2D_SIZE_U = D2D_SIZE_U { width: (rc.right - rc.left) as u32, height: (rc.bottom - rc.top) as u32 };

            unsafe { self.render_target.as_ref().unwrap().Resize(&size).expect("Failed at Resizing the window.")};
            self.calculate_layout();
            unsafe {InvalidateRect(self.window_handle, std::ptr::null_mut(), BOOL(0))};
        }
    }
}

fn create_factory() -> Result<ID2D1Factory> {
    // Contains the debugging level of an ID2D1Factory object.
    let mut options = D2D1_FACTORY_OPTIONS::default();

    // `debug_assertions` are enabled by default when compiling without
    // optimizations. This can be used to enable extra debugging code in
    // development but not in production. For example, it controls the
    // behavior of the standard library's debug_assert! macro.
    if cfg!(debug_assertions) {
        // Direct2D sends error messages, warnings, and additional diagnostic
        // information that can help improve performance to the debug
        // layer.
        options.debugLevel = D2D1_DEBUG_LEVEL_INFORMATION;
    }

    let mut result = None;
    unsafe {
        // Creates a factory object that can be used to create Direct2D
        // resources
        D2D1CreateFactory(
            // The threading model of the factory and the resources it creates.
            // You can specify whether it is multithreaded or singlethreaded.
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            &ID2D1Factory::IID,
            &options,
            // The address to a pointer to the new factory
            std::mem::transmute(&mut result),
        )
        // when an Ok(()) was returned, we expect result contains the new
        // factory
        .map(|()| result.unwrap())
    }
}
fn main() -> Result<()> {
    unsafe {
        // Handle to an instance" or "handle to a module." The operating system
        // uses this value to identify the executable (EXE) when it is
        // loaded in memory. `GetModuleHandleW` retrieves a module
        // handle for the specified module. The module must have been
        // loaded by the calling process. If the lpmodulename parameter is
        // `None`, GetModuleHandle returns a handle to the file used to create
        // the calling process (.exe file). If the function succeeds, the return
        // value is a handle to the specified module. If the function fails, the
        // return value is NULL.
        // https://docs.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlew
        let instance = GetModuleHandleW(None);
        debug_assert!(instance.0 != 0);

        let window_class = b"window\0";

        let window_context: *const WindowContext = &WindowContext::new()?;

        // Fields
        // style: WNDCLASS_STYLES
        // lpfnWndProc: Option<WNDPROC>
        // cbClsExtra: i32
        // cbWndExtra: i32
        // hInstance: HINSTANCE
        // hIcon: HICON
        // hCursor: HCURSOR
        // hbrBackground: HBRUSH
        // lpszMenuName: PWSTR
        // lpszClassName: PWSTR
        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-wndclassw
        let wc = WNDCLASSA {
            hInstance: instance,
            lpszClassName: PSTR(window_class.as_ptr() as *mut u8),
            lpfnWndProc: Some(wndproc),
            // https://docs.microsoft.com/en-us/windows/win32/winmsg/window-class-styles
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
            // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadcursorw
            hCursor: LoadCursorW(None, IDC_CROSS),

            ..Default::default()
        };
        // Registers a window class for subsequent use in calls to the
        // CreateWindow or CreateWindowEx function.
        // If the function succeeds, the return value is a class atom that
        // uniquely identifies the class being registered. This atom can be
        // used by the CreateWindowEx function.
        let atom = RegisterClassA(&wc);
        debug_assert!(atom != 0);

        // Create the window.
        // returns a handle to the new window, or zero if the function fails.
        // https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowexw
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
            // A pointer to arbitrary data of type void*. You can use this
            // value to pass a data structure to your window procedure.
            window_context as *const c_void,
        );
        debug_assert!(handle.0 != 0);

        let mut message = MSG::default();

        // Retrieves a message from the calling thread's message queue. The
        // function dispatches incoming sent messages until a posted message is
        // available for retrieval. Unlike GetMessage, the PeekMessage
        // function does not wait for a message to be posted before returning.
        // If the hwnd parameter is `0`, GetMessage retrieves messages for any
        // window that belongs to the current thread, and any messages
        // on the current thread's message queue whose hwnd value is `0`.
        // Therefore if hWnd is `0`, both window messages and thread
        // messages are processed.
        while GetMessageA(&mut message, HWND(0), 0, 0).into() {
            // TranslateMessage(&mut message);
            // Dispatches a message to a window procedure.
            DispatchMessageA(&mut message);
        }

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
        // Get the initial application data when the window is first created
        // (via CreateWindowEx function).
        if message == WM_CREATE {
            let createstruct = &mut *(lparam.0 as *mut CREATESTRUCTA);
            let init_context: &mut WindowContext =
                mem::transmute(createstruct.lpCreateParams);

            // Register the window handle in our user controlled context data.
            init_context.window_handle = Some(hwnd);

            // Pass the pointer of the user data structure to the window
            // instance. From then on you can always retrieve the pointer back
            // from the window by calling the GetWindowLongPtrA function.
            SetWindowLongPtrA(
                hwnd,
                GWLP_USERDATA,
                mem::transmute(init_context),
            );
        }

        // Retrieve the user data associated with this window instance.
        let window_context: &mut WindowContext = {
            let user_data_ = GetWindowLongPtrA(hwnd, GWLP_USERDATA);
            mem::transmute(user_data_)
        };

        match message as u32 {
            WM_PAINT => {
                println!("WM_PAINT");
                window_context.on_paint();
                LRESULT(0)
            }

            WM_SIZE => {
                window_context.resize();
                
                LRESULT(0)
            }

            WM_DESTROY => {
                println!("WM_DESTROY");
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcA(hwnd, message, wparam, lparam),
        }
    }
}
