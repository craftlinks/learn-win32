use windows::core::Result;
use windows::Win32::Foundation::{HWND, PWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL,
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK};

fn main() -> Result<()> {
    unsafe {
        // Initializes the COM library for use by the calling thread, sets the
        // thread's concurrency model, and creates a new apartment for the
        // thread if one is required.

        // NOTE Geert: You should call Windows::Foundation::Initialize to
        // initialize the thread instead of CoInitializeEx if you want
        // to use the Windows Runtime APIs or if you want to use both
        // COM and Windows Runtime components. Windows::Foundation::
        // Initialize is sufficient to use for COM components.

        CoInitializeEx(
            std::ptr::null_mut(),
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        )?;

        // HINT: https://github.com/microsoft/windows-samples-rs/search?q=CoCreateInstance

        // Create the FileOpenDialog object.
        //
        let file_open: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL)?;

        // Launches the modal window.
        file_open.Show(None)?;

        // Gets the choice that the user made in the dialog.
        let item = file_open.GetResult()?;

        // Gets the display name of the IShellItem object.
        let file_path = item.GetDisplayName(SIGDN_FILESYSPATH)?;

        // Launch a simple message box to show the file path of the previously
        // selected path.
        MessageBoxW(
            HWND(0),
            file_path,
            // Geert TODO: Figure out how to go from UTF-8 to UTF-16 here to
            // avoid gibberish text.
            PWSTR("File Path\0".as_ptr() as *mut u16),
            MB_OK,
        );

        // Closes the COM library on the current thread, unloads all DLLs loaded
        // by the thread, frees any other resources that the thread maintains,
        // and forces all RPC connections on the thread to close.
        CoUninitialize();
    }
    Ok(())
}
