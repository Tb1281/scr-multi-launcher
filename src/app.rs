use std::{collections::BTreeSet, time::Duration};

use iced::{
    Background, Border, Color, Element, Font, Length, Subscription, Task, border,
    widget::{
        Space, button, center, column, container, mouse_area, opaque, row, scrollable, stack, svg,
        text,
    },
};
use windows::Win32::System::Threading::TerminateProcess;

use crate::{
    KOREAN_FONT, SCRStruct,
    scr::{StringExt, get_owned_handle, get_path, process_handles, query_child, run_scr, save_log},
    settings::Settings,
};

const CHECK_INTERVAL: Duration = Duration::from_millis(500);
const GEAR: &[u8] = include_bytes!("../assets/gear-svgrepo-com.svg");
const SEARCH: &[u8] = include_bytes!("../assets/search-svgrepo-com.svg");

#[derive(Debug, Clone)]
pub enum Message {
    Tick,

    ProcessCheckResult(Vec<SCRStruct>, Vec<String>),
    ProcessLog(Option<String>),
    KillAll,
    RunSCR(String),
    SaveLogs,
    ClearLogs,
    OpenSettings,
    CmdResult(Result<(), String>),

    // 설정 다이얼로그 메시지
    OpenFolderDialog32,
    OpenFolderDialog64,
    SaveSettings,
    SaveSettingsResult(Result<Settings, String>),
    CloseSettings,
}

pub struct App {
    is_timer_on: bool,
    show_settings: bool,
    childs: BTreeSet<SCRStruct>,
    logs: BTreeSet<String>,
    settings: Settings,
    temp_settings: Settings,
}

impl App {
    pub fn new(maybe_settings: Option<Settings>) -> Self {
        let childs = BTreeSet::new();
        let mut logs = BTreeSet::new();
        let settings = if let Some(settings) = maybe_settings {
            settings
        } else {
            logs.insert("conf.toml 파일이 없거나 손상되었습니다.".as_log());
            Settings::default()
        };
        let temp_settings = settings.clone();

        Self {
            is_timer_on: true,
            show_settings: false,
            childs,
            settings,
            temp_settings,
            logs,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(CHECK_INTERVAL).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let childs = std::mem::take(&mut self.childs);
                Task::perform(
                    async move {
                        let mut new_childs = process_handles();
                        let mut logs = Vec::new();
                        new_childs.extend(childs);

                        new_childs.retain_mut(|child| {
                            if child.is_processed {
                                if get_owned_handle(child.pid).is_none() {
                                    logs.push(format!("Invalid PID: {}", child.pid).as_log());

                                    false
                                } else {
                                    true
                                }
                            } else if let Some(log) = query_child(child.pid, None) {
                                child.is_processed = true;
                                logs.push(log);

                                true
                            } else {
                                true
                            }
                        });

                        (new_childs, logs)
                    },
                    |(new_childs, logs)| Message::ProcessCheckResult(new_childs, logs),
                )
            }
            Message::ProcessCheckResult(new_childs, logs) => {
                self.childs.extend(new_childs);
                self.logs.extend(logs);
                if !self.is_timer_on {
                    self.kill_childs();
                    self.is_timer_on = true;
                }

                Task::none()
            }
            Message::RunSCR(path) => Task::perform(
                async move {
                    if let Some((pid, handle)) = run_scr(&path, &["-launch"]) {
                        let log = query_child(pid, Some(handle));

                        log
                    } else {
                        None
                    }
                },
                |maybe_log| Message::ProcessLog(maybe_log),
            ),
            Message::KillAll => {
                self.is_timer_on = false;
                self.kill_childs();

                Task::none()
            }
            Message::ProcessLog(maybe_log) => {
                if let Some(log) = maybe_log {
                    self.logs.insert(log.as_log());
                }

                Task::none()
            }
            Message::OpenFolderDialog32 => {
                if let Some(path_32) = get_path() {
                    self.temp_settings.path_32 = path_32;
                }

                Task::none()
            }
            Message::OpenFolderDialog64 => {
                if let Some(path_64) = get_path() {
                    self.temp_settings.path_64 = path_64;
                }

                Task::none()
            }
            Message::SaveLogs => {
                let logs = std::mem::take(&mut self.logs);
                Task::perform(save_log(logs), Message::CmdResult)
            }
            Message::CmdResult(result) => {
                if let Err(err) = result {
                    self.logs.insert(err.as_log());
                }
                Task::none()
            }
            Message::OpenSettings => {
                self.show_settings = true;
                self.temp_settings = self.settings.clone();

                iced::widget::focus_next()
            }
            Message::SaveSettings => {
                self.show_settings = false;

                Task::perform(
                    std::mem::take(&mut self.temp_settings).save(),
                    Message::SaveSettingsResult,
                )
            }
            Message::SaveSettingsResult(result) => {
                match result {
                    Ok(settings) => self.settings = settings,
                    Err(err) => {
                        self.logs.insert(err.as_log());
                    }
                }

                Task::none()
            }
            Message::CloseSettings => {
                self.show_settings = false;

                Task::none()
            }
            Message::ClearLogs => {
                self.logs.clear();

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let content = self.main_view();
        if self.show_settings {
            modal(content, self.settings_view(), Message::CloseSettings)
        } else {
            content.into()
        }
    }

    fn kill_childs(&mut self) -> () {
        self.childs.retain(|child| {
            if let Some(handle) = get_owned_handle(child.pid) {
                match unsafe { TerminateProcess(*handle, 0) } {
                    Ok(_) => {
                        self.logs.insert(
                            format!("Successfully terminated process wid PID {}", child.pid)
                                .as_log(),
                        );
                        false
                    }
                    _ => true,
                }
            } else {
                false
            }
        });
    }

    fn main_view(&self) -> Element<Message> {
        let run_32 = if self.settings.path_32.is_empty() {
            button("32bit").padding([8, 24])
        } else {
            button("32bit")
                .on_press(Message::RunSCR(self.settings.path_32.clone()))
                .padding([8, 24])
        };

        let run_64 = if self.settings.path_64.is_empty() {
            button("64bit").padding([8, 24])
        } else {
            button("64bit")
                .on_press(Message::RunSCR(self.settings.path_32.clone()))
                .padding([8, 24])
        };

        // 상단 버튼 행
        let top_row = row![
            button(
                svg(iced::widget::svg::Handle::from_memory(GEAR)).style(|_, _| svg::Style {
                    color: Some(Color::WHITE)
                })
            )
            .width(36)
            .height(36)
            .padding(2)
            .on_press(Message::OpenSettings),
            Space::with_width(Length::Fill),
            run_32,
            Space::with_width(Length::Fixed(12.0)),
            run_64,
            Space::with_width(Length::Fixed(12.0)),
            button("Kill All")
                .on_press(Message::KillAll)
                .padding([8, 16]),
        ]
        .align_y(iced::Alignment::Center);

        // 로그 영역
        let logs_colum = column(self.logs.iter().map(|log| text(log).size(12).into()));
        let logs_area = container(scrollable(logs_colum))
            .style(|_theme| container::Style {
                background: Some(Background::Color(Color::WHITE)),
                border: Border {
                    color: Color::BLACK,
                    width: 1.0,
                    radius: border::Radius::new(0),
                },
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(4);

        // 하단 버튼 행
        let bottom_row = row![
            button(text("Save Logs").center())
                .on_press(Message::SaveLogs)
                .width(Length::Fill)
                .padding(8),
            Space::with_width(Length::Fixed(8.0)),
            button(text("Clear Logs").center())
                .on_press(Message::ClearLogs)
                .width(Length::Fill)
                .padding(8),
        ];

        column![top_row, logs_area, bottom_row,]
            .padding(8)
            .spacing(8)
            .into()
    }

    fn settings_view(&self) -> Element<Message> {
        let dialog_content = column![
            text("설정").size(18).font(Font {
                weight: iced::font::Weight::Bold,
                family: KOREAN_FONT.family,
                ..Default::default()
            }),
            Space::with_height(Length::Fixed(12.0)),
            text("32bit").size(16).font(Font {
                weight: iced::font::Weight::Bold,
                family: KOREAN_FONT.family,
                ..Default::default()
            }),
            row![
                container(text(&self.temp_settings.path_32).size(10))
                    .style(|_| container::Style {
                        border: Border {
                            color: Color::BLACK,
                            width: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .width(Length::Fill)
                    .padding([8, 4]),
                button(
                    svg(iced::widget::svg::Handle::from_memory(SEARCH)).style(|_, _| {
                        svg::Style {
                            color: Some(Color::WHITE),
                        }
                    })
                )
                .width(36)
                .height(36)
                .padding(4)
                .on_press(Message::OpenFolderDialog32),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
            Space::with_height(Length::Fixed(8.0)),
            text("64bit").font(Font {
                weight: iced::font::Weight::Bold,
                family: KOREAN_FONT.family,
                ..Default::default()
            }),
            row![
                container(text(&self.temp_settings.path_64).size(10))
                    .style(|_| container::Style {
                        border: Border {
                            color: Color::BLACK,
                            width: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .width(Length::Fill)
                    .padding([8, 4]),
                button(
                    svg(iced::widget::svg::Handle::from_memory(SEARCH)).style(|_, _| svg::Style {
                        color: Some(Color::WHITE)
                    })
                )
                .width(36)
                .height(36)
                .padding(4)
                .on_press(Message::OpenFolderDialog64),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
            Space::with_height(Length::Fixed(12.0)),
            row![
                Space::with_width(Length::Fill),
                button("확인")
                    .on_press(Message::SaveSettings)
                    .padding([8, 16]),
                Space::with_width(Length::Fixed(16.0)),
                button("취소")
                    .on_press(Message::CloseSettings)
                    .padding([8, 16]),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .padding(8);

        // 배경 오버레이
        let overlay = container(dialog_content)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::WHITE)),
                ..Default::default()
            })
            .width(Length::Fixed(360.0));

        overlay.into()
    }
}

fn modal<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_blur: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    stack![
        base.into(),
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| {
                container::Style {
                    background: Some(
                        Color {
                            a: 0.8,
                            ..Color::BLACK
                        }
                        .into(),
                    ),
                    ..container::Style::default()
                }
            }))
            .on_press(on_blur)
        )
    ]
    .into()
}
