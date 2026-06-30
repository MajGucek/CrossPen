use crate::PenData;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

static mut LAST_DOWN: bool = false;
static mut LAST_X: i32 = 0;
static mut LAST_Y: i32 = 0;

pub fn run(pen: &PenData) {
    unsafe {
        let current_x = (pen.x.clamp(0.0, 1.0) * 65535.0) as i32;
        let current_y = (pen.y.clamp(0.0, 1.0) * 65535.0) as i32;

        let mut inputs: Vec<INPUT> = Vec::new();

        if LAST_X != 0 || LAST_Y != 0 {
            let dx = current_x - LAST_X;
            let dy = current_y - LAST_Y;
            if dx != 0 || dy != 0 {
                inputs.push(move_relative(dx, dy));
            }
        }

        if pen.button_1 && !LAST_DOWN {
            inputs.push(mouse(MOUSEEVENTF_LEFTDOWN));
            LAST_DOWN = true;
        } else if !pen.button_1 && LAST_DOWN {
            inputs.push(mouse(MOUSEEVENTF_LEFTUP));
            LAST_DOWN = false;
        }

        LAST_X = current_x;
        LAST_Y = current_y;

        if !inputs.is_empty() {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

fn move_relative(dx: i32, dy: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn mouse(flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}