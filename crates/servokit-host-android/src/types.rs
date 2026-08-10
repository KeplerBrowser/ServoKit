#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServoStatus {
    Ok = 0,
    InvalidUrl = 1,
    NullPointer = 2,
    UnsupportedTarget = 3,
    BackendError = 4,
    InvalidControllerHandle = 5,
    ExpiredControllerHandle = 6,
}
