#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod scr;
mod settings;

use crate::{app::App, scr::get_mutex, settings::Settings};
use iced::{Font, Task};

const APP_NAME: &str = "SC:R Multi-Launcher";
const KOREAN_FONT: Font = Font::with_name("Malgun Gothic");

fn main() -> iced::Result {
    let title_fn = |_: &App| String::from(APP_NAME);
    if get_mutex() {
        iced::application(
            || {
                let settings = iced::futures::executor::block_on(Settings::load());
                (App::new(settings), Task::none())
            },
            App::update,
            App::view,
        )
        .title(title_fn)
        .subscription(App::subscription)
        .default_font(KOREAN_FONT)
        .window_size((400.0, 300.0))
        .resizable(false)
        .run()
    } else {
        Ok(())
    }
}
