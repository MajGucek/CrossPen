pub mod stylus {
    use crate::PenData;
    use std::sync::{Mutex, OnceLock};
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use windows::Win32::UI::WindowsAndMessaging::{LoadCursorW, CopyImage, SetSystemCursor, SystemParametersInfoW, IDC_HAND, IMAGE_CURSOR, IMAGE_FLAGS, OCR_NORMAL, SPI_SETCURSORS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, HCURSOR};
    use windows::Win32::Foundation::HANDLE;

    // THIS CODE IS AI GENERATED, if you think I'm going to spend hours researching non-existing documentation, you're wrong.
    // I'm actually kind of impressed this just doesn't implode since there is so much unsafe ;)

    const SENSITIVITY: f32 = 0.05;
    const SMOOTHING: f32 = 0.3;

    #[derive(Default)]
    struct PenState {
        last_down: bool,
        has_baseline: bool,
        last_x: f32,
        last_y: f32,
        smooth_x: f32,
        smooth_y: f32,
        acc_x: f32,
        acc_y: f32,
    }

    fn state() -> &'static Mutex<PenState> {
        static STATE: OnceLock<Mutex<PenState>> = OnceLock::new();
        STATE.get_or_init(|| Mutex::new(PenState::default()))
    }

    pub fn run(pen: &PenData) {
        let mut st = state().lock().unwrap();

        if !pen.is_touching {
            if st.last_down {
                send_input(&[mouse_event(MOUSEEVENTF_LEFTUP)]);
                st.last_down = false;
                set_grab_cursor(false);
            }
            st.has_baseline = false;
            st.acc_x = 0.0;
            st.acc_y = 0.0;
            return;
        }

        let raw_x = pen.x.clamp(0.0, 1.0) * 65535.0;
        let raw_y = pen.y.clamp(0.0, 1.0) * 65535.0;

        if st.has_baseline {
            st.smooth_x += (raw_x - st.smooth_x) * (1.0 - SMOOTHING);
            st.smooth_y += (raw_y - st.smooth_y) * (1.0 - SMOOTHING);
        } else {
            st.smooth_x = raw_x;
            st.smooth_y = raw_y;
        }

        let mut inputs: Vec<INPUT> = Vec::new();

        if st.has_baseline {
            let dx = (st.smooth_x - st.last_x) * SENSITIVITY + st.acc_x;
            let dy = (st.smooth_y - st.last_y) * SENSITIVITY + st.acc_y;

            let move_x = dx.trunc() as i32;
            let move_y = dy.trunc() as i32;

            st.acc_x = dx - move_x as f32;
            st.acc_y = dy - move_y as f32;

            if move_x != 0 || move_y != 0 {
                inputs.push(move_relative(move_x, move_y));
            }
        } else {
            st.has_baseline = true;
        }

        if pen.button_1 && !st.last_down {
            inputs.push(mouse_event(MOUSEEVENTF_LEFTDOWN));
            st.last_down = true;
            set_grab_cursor(true);
        } else if !pen.button_1 && st.last_down {
            inputs.push(mouse_event(MOUSEEVENTF_LEFTUP));
            st.last_down = false;
            set_grab_cursor(false);
        }

        st.last_x = st.smooth_x;
        st.last_y = st.smooth_y;

        if !inputs.is_empty() {
            send_input(&inputs);
        }
    }

    fn send_input(inputs: &[INPUT]) {
        unsafe {
            let _ = SendInput(inputs, size_of::<INPUT>() as i32);
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

    fn mouse_event(flags: MOUSE_EVENT_FLAGS) -> INPUT {
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



    fn set_grab_cursor(enabled: bool) {
        unsafe {
            if enabled {
                if let Ok(hcursor) = LoadCursorW(None, IDC_HAND) {
                    let handle = HANDLE(hcursor.0);
                    if let Ok(copy) = CopyImage(handle, IMAGE_CURSOR, 0, 0, IMAGE_FLAGS(0)) {
                        let copy_cursor = HCURSOR(copy.0);
                        let _ = SetSystemCursor(copy_cursor, OCR_NORMAL);
                    }
                }
            } else {
                let _ = SystemParametersInfoW(
                    SPI_SETCURSORS,
                    0,
                    None,
                    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                );
            }
        }
    }
}
