/// A package identity also protects MSIX builds made outside the Store script.
/// Fail closed on unexpected Windows API errors instead of enabling NSIS updates.
pub fn store_managed() -> bool {
    use windows_sys::Win32::{
        Foundation::APPMODEL_ERROR_NO_PACKAGE, Storage::Packaging::Appx::GetCurrentPackageFullName,
    };
    let mut length = 0;
    cfg!(feature = "microsoft-store")
        || unsafe { GetCurrentPackageFullName(&mut length, std::ptr::null_mut()) }
            != APPMODEL_ERROR_NO_PACKAGE
}

#[cfg(all(test, feature = "microsoft-store"))]
mod tests {
    #[test]
    fn store_build_is_store_managed_even_without_package_identity() {
        assert!(super::store_managed());
    }
}
