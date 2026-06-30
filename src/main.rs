use std::net::UdpSocket;
use std::thread::JoinHandle;
use std::time::Duration;
use crossbeam_channel::{select, Receiver, Sender};
use eframe::egui::{CentralPanel, Direction, Event, Layout, TextBuffer, Ui, ViewportCommand, Visuals};
use eframe::{CreationContext, Frame, NativeOptions};
use env_logger::Env;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct PenData {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub is_touching: bool,
}


#[derive(Debug, Default)]
struct MainScreen {}
impl MainScreen {
    pub fn startup(&mut self) {}
    pub fn run_ui(&mut self, ui: &mut Ui) -> Option<AppState> {
        let mut next_state = None;
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            ui.vertical_centered(|ui| {
                if ui.button("Sender").clicked() {
                    next_state = Some(AppState::Sender(SenderScreen::default()));
                }
                if ui.button("Receiver").clicked() {
                    next_state = Some(AppState::Receiver(ReceiverScreen::default()));
                }
            });

        });
        next_state
    }
}
#[derive(Debug, Default)]
struct SenderScreen {
    thread_handle: Option<JoinHandle<()>>,
    kill_signal: Option<Sender<()>>,
    data_signal: Option<Sender<String>>,
}
impl SenderScreen {
    pub fn startup(&mut self) {
        let (kill_tx, kill_rx) = crossbeam_channel::bounded::<()>(0);
        let (data_tx, data_rx) = crossbeam_channel::bounded::<String>(100);
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        socket.set_broadcast(true).unwrap();
        let handle = std::thread::spawn(move || {
            log::info!("Sender background thread spawned");
            loop {
                select! {
                    recv(data_rx) -> msg => {
                        if let Ok(message_to_send) = msg {
                            log::info!("Sending: {:?}", message_to_send.as_str());
                            socket.send_to(message_to_send.as_bytes(), "255.255.255.255:8888");
                        }
                    }
                    recv(kill_rx) -> _ => {
                        break;
                    }
                }
            }
            log::info!("exited sender thread");
        });
        self.thread_handle = Some(handle);
        self.kill_signal = Some(kill_tx);
        self.data_signal = Some(data_tx);
    }
    fn shutdown(&mut self) {
        if let Some(ref killer) = self.kill_signal {
            let _ = killer.send(());
        }
        self.kill_signal = None;
        self.data_signal = None;
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
    pub fn run_ui(&mut self, ui: &mut Ui) -> Option<AppState> {
        let mut next_state = None;

        let mut current_pen = PenData::default();
        let mut pen_active = false;

        ui.ctx().input(|i| {
            if let Some(pos) = i.pointer.latest_pos() {
                let rect = ui.ctx().screen_rect();
                current_pen.x = pos.x / rect.width();
                current_pen.y = pos.y / rect.height();
                pen_active = true;
            }
            current_pen.is_touching = i.pointer.any_down();
            if current_pen.is_touching {
                current_pen.pressure = 1.;
            }

            for event in &i.events {
                if let Event::Touch { force: Some(f), .. } = event {
                    current_pen.pressure = *f;
                }
            }
        });

        if pen_active {
            if let Some(ref sender) = self.data_signal {
                if let Ok(json) = serde_json::to_string(&current_pen) {
                    let _ = sender.send(json);
                }
            }
        }

        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            ui.vertical_centered(|ui| {
                if ui.button("test").clicked() {
                }
                if ui.button("Exit").clicked() {
                    self.shutdown();
                    next_state = Some(AppState::Main(MainScreen {}));
                }
            });
        });



        next_state
    }
}

#[derive(Debug, Default)]
struct ReceiverScreen {
    thread_handle: Option<JoinHandle<()>>,
    kill_signal: Option<Sender<()>>,
    data_receiver: Option<Receiver<String>>,
    pen_data: PenData,
}
impl ReceiverScreen {
    pub fn startup(&mut self) {
        let (kill_tx, kill_rx) = crossbeam_channel::bounded::<()>(1);
        let (data_tx, data_rx) = crossbeam_channel::unbounded::<String>();
        let socket = UdpSocket::bind("0.0.0.0:8888").unwrap();
        //socket.set_read_timeout(Some(std::time::Duration::from_millis(100))).unwrap();
        let handle = std::thread::spawn(move || {
            log::info!("Receiver background thread spawned");
            let mut buf = [0; 1024];
            loop {
                if kill_rx.try_recv().is_ok() {
                    break;
                }
                match socket.recv_from(&mut buf) {
                    Ok((amt, src)) => {
                        let _ = data_tx.send(String::from_utf8_lossy(&buf[..amt]).to_string());
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                        continue;
                    }
                    Err(e) => {
                        log::error!("Real network error: {}", e);
                        break;
                    }
                }
            }
            log::info!("exited receiver thread");
        });
        self.thread_handle = Some(handle);
        self.kill_signal = Some(kill_tx);
        self.data_receiver = Some(data_rx);
    }
    fn shutdown(&mut self) {
        if let Some(ref killer) = self.kill_signal {
            let _ = killer.send(());
        }
        self.kill_signal = None;
        self.data_receiver = None;

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
    pub fn run_ui(&mut self, ui: &mut Ui) -> Option<AppState> {
        let mut next_state = None;
        if let Some(ref receiver) = self.data_receiver {
            while let Ok(msg) = receiver.try_recv() {
                self.pen_data = serde_json::from_str::<PenData>(msg.as_str()).unwrap_or(PenData::default());
            }
        }
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            ui.vertical_centered(|ui| {
                ui.label(format!("{:?}", self.pen_data));
                if ui.button("Exit").clicked() {
                    self.shutdown();
                    next_state = Some(AppState::Main(MainScreen {}));
                }
            });
        });



        next_state
    }
}


#[derive(Debug)]
enum AppState {
    Main(MainScreen),
    Sender(SenderScreen),
    Receiver(ReceiverScreen),
}
impl AppState {
    fn startup(&mut self) {
        match self {
            AppState::Main(screen) => screen.startup(),
            AppState::Sender(screen) => screen.startup(),
            AppState::Receiver(screen) => screen.startup(),
        };
    }
    pub fn ui(&mut self, ui: &mut Ui) {
        let new_state: Option<AppState> =
        match self {
            AppState::Main(screen) => screen.run_ui(ui),
            AppState::Sender(screen) => screen.run_ui(ui),
            AppState::Receiver(screen) => screen.run_ui(ui),
        };
        new_state.map(|state| {
            *self = state;
            self.startup();
        });
    }
}

struct App {
    state: AppState,
}

impl App {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Self {
        cc.egui_ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
        cc.egui_ctx.set_visuals(Visuals::dark());
        Self {state: AppState::Main(MainScreen {})}
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, frame: &mut Frame) {
        ui.request_repaint_after(Duration::from_millis(1));
        self.state.ui(ui);
    }
}

fn main() {
    env_logger::Builder::from_env(
        Env::default().default_filter_or("info")
    ).init();

    log::info!("App started");
    eframe::run_native("Cross Pen",
    NativeOptions::default(),
    Box::new(|cc| Ok(Box::new(App::new(cc))))
    ).unwrap();
}
