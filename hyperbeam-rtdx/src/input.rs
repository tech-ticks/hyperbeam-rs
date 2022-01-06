use pmdrtdx_bindings as pmd;
use std::ptr::{null, null_mut};
use pmdrtdx_bindings::InputSystem_TouchParameter;

static mut INPUT_SYSTEM: *mut pmd::InputSystem = null_mut();

#[derive(Copy, Clone)]
pub enum Button {
    A = 1,
    B = 2,
    X = 1024,
    Y = 2028,
    R = 256,
    L = 512,
    ZR = 4096,
    ZL = 8192,
    SR = 16384,
    SL = 32768,
    Select = 4,
    Start = 8,
    Right = 16,
    Left = 32,
    Up = 64,
    Down = 128,
    All = 65535,
    AutoWalkCancel = 53247,
    Dir = 240,
    UpR = 262144,
    DownR = 524288,
    ShortcutRight = 1048576,
    ShortcutLeft = 2097152,
    ShortcutUp = 4194304,
    ShortcutDown = 8388608
}

pub fn get_button_down(button: Button) -> bool {
    get_input_system_instance().lastPadDown_ & button as u32 != 0
}

pub fn get_button_up(button: Button) -> bool {
    get_input_system_instance().lastPadUp_ & button as u32 != 0
}

pub fn get_button(button: Button) -> bool {
    get_input_system_instance().lastPadData_ & button as u32 != 0
}

pub fn get_button_repeat(button: Button) -> bool {
    get_input_system_instance().lastPadRepeat_ & button as u32 != 0
}

pub fn get_left_stick() -> pmd::Vector2 {
    get_input_system_instance().lastAnalogL
}

pub fn get_right_stick() -> pmd::Vector2 {
    get_input_system_instance().lastAnalogR
}

pub fn force_update() {
    unsafe {
        pmd::InputSystem_Update(get_input_system_instance() as _, true, null_mut());
    }
}

fn get_input_system_instance() -> &'static mut pmd::InputSystem {
    unsafe {
        if INPUT_SYSTEM.is_null() {
            INPUT_SYSTEM = pmd::Singleton_1_InputSystem__get_Instance(pmd::Singleton_1_InputSystem__get_Instance__MethodInfo);
        }
        INPUT_SYSTEM.as_mut().unwrap()
    }
}
