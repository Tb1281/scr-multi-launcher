use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use iced::{
    Background, Border, Color, Element, Font, Length, Subscription, Task, border,
    widget::{
        Space, button, center, column, container, mouse_area, opaque, row, scrollable, stack, svg,
        text,
    },
};

use crate::{
    KOREAN_FONT,
    scr::{
        StringExt, check_processes_worker, get_path_async, kill_processes_async, run_scr_async,
        save_log,
    },
    settings::Settings,
};

const CHECK_INTERVAL: Duration = Duration::from_millis(500);
const GEAR: &[u8] = include_bytes!("../assets/gear-svgrepo-com.svg");
const SEARCH: &[u8] = include_bytes!("../assets/search-svgrepo-com.svg");

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    ProcessCheckResult((BTreeMap<u32, bool>, Vec<String>)),
    ProcessLog(Vec<String>),
    KillAll,
    KilledAll((BTreeMap<u32, bool>, Vec<String>)),
    RunSCR(String),
    SaveLogs,
    LogsSaved(Result<BTreeSet<String>, String>),
    ClearLogs,
    OpenSettings,

    OpenFolderDialog32,
    PathSelected32(Option<String>),
    OpenFolderDialog64,
    PathSelected64(Option<String>),
    SaveSettings,
    SaveSettingsResult(Result<Settings, String>),
    CloseSettings,
}

pub struct App {
    is_timer_on: bool,
    is_checking: bool,
    show_settings: bool,
    childs: BTreeMap<u32, bool>, // 락(Lock) 없이 순수 데이터만 보유
    logs: BTreeSet<String>,
    settings: Settings,
    temp_settings: Settings,
}

impl App {
    pub fn new(maybe_settings: Result<Settings, String>) -> Self {
        let mut logs = BTreeSet::new();
        let settings = match maybe_settings {
            Ok(settings) => settings,
            Err(err) => {
                logs.insert(err.as_log());
                Settings::default()
            }
        };
        let temp_settings = settings.clone();

        Self {
            is_timer_on: true,
            is_checking: false,
            show_settings: false,
            childs: BTreeMap::new(),
            settings,
            temp_settings,
            logs,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.is_timer_on {
            iced::time::every(CHECK_INTERVAL).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if self.is_checking {
                    return Task::none();
                }

                self.is_checking = true;
                // UI에서 소유권을 완전히 빼서 워커로 던집니다. (Clone 없음)
                let childs = std::mem::take(&mut self.childs);

                Task::perform(check_processes_worker(childs), Message::ProcessCheckResult)
            }
            Message::ProcessCheckResult((returned_childs, logs)) => {
                self.is_checking = false;
                self.childs = returned_childs; // 워커가 돌려준 맵으로 원상복구
                self.logs.extend(logs);
                Task::none()
            }
            Message::RunSCR(path) => {
                let args = vec!["-launch".to_string()];
                Task::perform(run_scr_async(path, args), Message::ProcessLog)
            }
            Message::KillAll => {
                self.is_timer_on = false;
                let childs = std::mem::take(&mut self.childs);

                Task::perform(kill_processes_async(childs), Message::KilledAll)
            }
            Message::KilledAll((returned_childs, logs)) => {
                self.childs = returned_childs;
                self.logs.extend(logs);
                self.is_timer_on = true;
                Task::none()
            }
            Message::ProcessLog(logs) => {
                self.logs.extend(logs);
                Task::none()
            }
            Message::OpenFolderDialog32 => Task::perform(get_path_async(), Message::PathSelected32),
            Message::PathSelected32(maybe_path) => {
                if let Some(path) = maybe_path {
                    self.temp_settings.path_32 = path;
                }
                Task::none()
            }
            Message::OpenFolderDialog64 => Task::perform(get_path_async(), Message::PathSelected64),
            Message::PathSelected64(maybe_path) => {
                if let Some(path) = maybe_path {
                    self.temp_settings.path_64 = path;
                }
                Task::none()
            }
            Message::SaveLogs => {
                // logs의 소유권 이동 (Zero-Clone)
                let logs_to_save = std::mem::take(&mut self.logs);
                Task::perform(save_log(logs_to_save), Message::LogsSaved)
            }
            Message::LogsSaved(result) => {
                match result {
                    Ok(returned_logs) => {
                        let current_new_logs = std::mem::take(&mut self.logs);
                        self.logs = returned_logs;
                        self.logs.extend(current_new_logs);
                    }
                    Err(err) => {
                        self.logs.insert(err.as_log());
                    }
                }
                Task::none()
            }
            Message::OpenSettings => {
                self.show_settings = true;
                self.temp_settings = self.settings.clone();
                Task::none() // Iced 0.14: 불필요한 focus_next 제거
            }
            Message::SaveSettings => {
                self.show_settings = false;
                // 디스크 저장이 실패하더라도 이번 세션에서는 설정이 적용되도록 즉시 반영한다.
                self.settings = self.temp_settings.clone();
                Task::perform(self.settings.clone().save(), Message::SaveSettingsResult)
            }
            Message::SaveSettingsResult(result) => {
                if let Err(err) = result {
                    self.logs.insert(err.as_log());
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

    pub fn view(&self) -> Element<'_, Message> {
        let content = self.main_view();
        if self.show_settings {
            modal(content, self.settings_view(), Message::CloseSettings)
        } else {
            content
        }
    }

    fn main_view(&self) -> Element<'_, Message> {
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
                .on_press(Message::RunSCR(self.settings.path_64.clone()))
                .padding([8, 24])
        };

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
            Space::new().width(Length::Fill),
            run_32,
            Space::new().width(Length::Fixed(12.0)),
            run_64,
            Space::new().width(Length::Fixed(12.0)),
            button("Kill All")
                .on_press(Message::KillAll)
                .padding([8, 16]),
        ]
        .align_y(iced::Alignment::Center);

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

        let bottom_row = row![
            button(text("Save Logs").center())
                .on_press(Message::SaveLogs)
                .width(Length::Fill)
                .padding(8),
            Space::new().width(Length::Fixed(8.0)),
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

    fn settings_view(&self) -> Element<'_, Message> {
        let dialog_content = column![
            text("설정").size(18).font(Font {
                weight: iced::font::Weight::Bold,
                family: KOREAN_FONT.family,
                ..Default::default()
            }),
            Space::new().height(Length::Fixed(12.0)),
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
                    svg(iced::widget::svg::Handle::from_memory(SEARCH)).style(|_, _| svg::Style {
                        color: Some(Color::WHITE)
                    })
                )
                .width(36)
                .height(36)
                .padding(4)
                .on_press(Message::OpenFolderDialog32),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
            Space::new().height(Length::Fixed(8.0)),
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
            Space::new().height(Length::Fixed(12.0)),
            row![
                Space::new().width(Length::Fill),
                button("확인")
                    .on_press(Message::SaveSettings)
                    .padding([8, 16]),
                Space::new().width(Length::Fixed(16.0)),
                button("취소")
                    .on_press(Message::CloseSettings)
                    .padding([8, 16]),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .padding(8);

        container(dialog_content)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::WHITE)),
                ..Default::default()
            })
            .width(Length::Fixed(360.0))
            .into()
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
