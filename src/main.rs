#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod scr;
mod settings;

use std::cmp::Ordering;

use iced::{Font, Task};

use crate::{app::App, scr::get_mutex, settings::Settings};

const APP_NAME: &str = "SC:R Multi-Launcher";
const KOREAN_FONT: Font = Font::with_name("Malgun Gothic");

#[derive(Debug, Clone)]
struct SCRStruct {
    pid: u32,
    is_processed: bool,
}

// Eq + PartialEq: pid만 비교
impl PartialEq for SCRStruct {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}
impl Eq for SCRStruct {}

// Ord + PartialOrd: pid만 비교
impl PartialOrd for SCRStruct {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.pid.cmp(&other.pid))
    }
}
impl Ord for SCRStruct {
    fn cmp(&self, other: &Self) -> Ordering {
        self.pid.cmp(&other.pid)
    }
}

impl SCRStruct {
    fn new(pid: u32) -> Self {
        Self {
            pid,
            is_processed: false,
        }
    }
}

fn main() -> iced::Result {
    if get_mutex() {
        iced::application(APP_NAME, App::update, App::view)
            .subscription(App::subscription)
            .default_font(KOREAN_FONT)
            .window_size((400.0, 300.0))
            .resizable(false)
            .run_with(|| {
                let maybe_settings = iced::futures::executor::block_on(Settings::load());

                (App::new(maybe_settings), Task::none())
            })
    } else {
        Ok(())
    }
}
