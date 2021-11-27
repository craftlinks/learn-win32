use std::{error::Error, ffi::c_void, mem};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, PSTR, WPARAM},
    Graphics::Gdi::{BeginPaint, EndPaint, FillRect, HBRUSH, PAINTSTRUCT},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExA, DefWindowProcA, DispatchMessageA, GetMessageA,
        GetWindowLongPtrA, LoadCursorW, PostQuitMessage, RegisterClassA,
        SetWindowLongPtrA, COLOR_WINDOW, CREATESTRUCTA, CS_HREDRAW, CS_OWNDC,
        CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, IDC_CROSS, MSG, WM_CREATE,
        WM_DESTROY, WM_PAINT, WNDCLASSA, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    },
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Default)]
pub struct WindowState {
    i: u8,
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

        let window_state: *const WindowState = &WindowState { i: 100 };

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
            window_state as *const c_void,
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
            let initdata: &mut WindowState =
                mem::transmute(createstruct.lpCreateParams);

            // Pass the pointer of the user data structure to the window
            // instance. From then on you can always retrieve the pointer back
            // from the window by calling the GetWindowLongPtrA function.
            SetWindowLongPtrA(hwnd, GWLP_USERDATA, mem::transmute(initdata));
        }

        // Retrieve the user data associated with this window instance.
        let user_data: &mut WindowState = {
            let user_data_ = GetWindowLongPtrA(hwnd, GWLP_USERDATA);
            mem::transmute(user_data_)
        };

        match message as u32 {
            WM_PAINT => {
                println!("WM_PAINT");
                // The PAINTSTRUCT structure contains information for an
                // application. This information can be used to paint the client
                // area of a window owned by that application. This structure
                // will be filled in by the `BeginPaint` function.
                let mut ps = PAINTSTRUCT {
                    ..Default::default()
                };
                // The `BeginPaint` function prepares the specified window for
                // painting and fills a PAINTSTRUCT structure with information
                // about the painting.
                let hdc = BeginPaint(hwnd, &mut ps);
                debug_assert!(hdc.0 != 0);

                // The FillRect function fills the application client rectangle
                // by using the specified brush. This function includes the left
                // and top borders, but excludes the right and bottom borders of
                // the rectangle.
                let hbr = HBRUSH((COLOR_WINDOW.0 + 2).try_into().unwrap());
                FillRect(hdc, &ps.rcPaint, hbr);

                // The EndPaint function marks the end of painting in the
                // specified window. This function is required for each call to
                // the BeginPaint function, but only after painting is complete.
                EndPaint(hwnd, &ps);
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
