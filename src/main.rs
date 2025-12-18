use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_editor,
    text_input, Column,
};
use iced::{color, Element, Fill, Font, Task, Theme};
use std::time::{Duration, Instant};

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title("BadGateway")
        .theme(App::theme)
        .default_font(Font::MONOSPACE)
        .run()
}

fn sublime_palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: color!(0x1e1e2e),
        text: color!(0xcdd6f4),
        primary: color!(0x89b4fa),
        success: color!(0xa6e3a1),
        warning: color!(0xf9e2af),
        danger: color!(0xf38ba8),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Method {
    #[default]
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
    OPTIONS,
}

impl Method {
    const ALL: &'static [Method] = &[
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::HEAD,
        Method::OPTIONS,
    ];
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Method::GET => "GET",
                Method::POST => "POST",
                Method::PUT => "PUT",
                Method::PATCH => "PATCH",
                Method::DELETE => "DELETE",
                Method::HEAD => "HEAD",
                Method::OPTIONS => "OPTIONS",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RequestTab {
    #[default]
    Body,
    Headers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ResponseTab {
    #[default]
    Body,
    Headers,
}

#[derive(Debug, Clone)]
struct Response {
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    body: String,
    duration: Duration,
    size: usize,
}

#[derive(Default)]
struct App {
    url: String,
    method: Method,
    request_tab: RequestTab,
    response_tab: ResponseTab,
    request_body: text_editor::Content,
    request_headers: String,
    response: Option<Response>,
    loading: bool,
    error: Option<String>,
    history: Vec<HistoryEntry>,
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    method: Method,
    url: String,
    status: u16,
}

#[derive(Debug, Clone)]
enum Message {
    UrlChanged(String),
    MethodSelected(Method),
    RequestTabSelected(RequestTab),
    ResponseTabSelected(ResponseTab),
    RequestBodyChanged(text_editor::Action),
    RequestHeadersChanged(String),
    Send,
    ResponseReceived(Result<Response, String>),
    HistoryEntryClicked(usize),
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn theme(&self) -> Theme {
        Theme::custom("Catppuccin", sublime_palette())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UrlChanged(url) => {
                self.url = url;
                Task::none()
            }
            Message::MethodSelected(method) => {
                self.method = method;
                Task::none()
            }
            Message::RequestTabSelected(tab) => {
                self.request_tab = tab;
                Task::none()
            }
            Message::ResponseTabSelected(tab) => {
                self.response_tab = tab;
                Task::none()
            }
            Message::RequestBodyChanged(action) => {
                self.request_body.perform(action);
                Task::none()
            }
            Message::RequestHeadersChanged(headers) => {
                self.request_headers = headers;
                Task::none()
            }
            Message::Send => {
                self.loading = true;
                self.error = None;
                let url = self.url.clone();
                let method = self.method;
                let body = self.request_body.text();
                let headers = self.request_headers.clone();

                Task::perform(
                    async move { send_request(url, method, body, headers).await },
                    Message::ResponseReceived,
                )
            }
            Message::ResponseReceived(result) => {
                self.loading = false;
                match result {
                    Ok(response) => {
                        self.history.push(HistoryEntry {
                            method: self.method,
                            url: self.url.clone(),
                            status: response.status,
                        });
                        self.response = Some(response);
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                        self.response = None;
                    }
                }
                Task::none()
            }
            Message::HistoryEntryClicked(index) => {
                if let Some(entry) = self.history.get(index) {
                    self.url = entry.url.clone();
                    self.method = entry.method;
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let url_bar = self.view_url_bar();
        let main_content = row![
            self.view_request_panel(),
            self.view_response_panel()
        ]
        .spacing(1)
        .height(Fill);

        let content = column![url_bar, main_content].spacing(0).width(Fill);

        let main_view = row![
            self.view_sidebar(),
            content
        ]
        .spacing(1)
        .height(Fill);

        container(main_view)
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(color!(0x11111b).into()),
                ..Default::default()
            })
            .into()
    }

    fn view_sidebar(&self) -> Element<Message> {
        let title = text("History")
            .size(12)
            .color(color!(0x6c7086));

        let history_items: Vec<Element<Message>> = self
            .history
            .iter()
            .rev()
            .enumerate()
            .take(50)
            .map(|(i, entry)| {
                let status_color = match entry.status {
                    200..=299 => color!(0xa6e3a1),
                    400..=499 => color!(0xf9e2af),
                    500..=599 => color!(0xf38ba8),
                    _ => color!(0xcdd6f4),
                };

                let truncated_url = if entry.url.len() > 22 {
                    format!("{}...", &entry.url[..19])
                } else {
                    entry.url.clone()
                };

                let idx = self.history.len() - 1 - i;

                button(
                    column![
                        row![
                            text(entry.method.to_string()).size(10).color(color!(0x89b4fa)),
                            text(entry.status.to_string()).size(10).color(status_color),
                        ].spacing(8),
                        text(truncated_url).size(11).color(color!(0x6c7086)),
                    ]
                    .spacing(2),
                )
                .width(Fill)
                .padding(8)
                .style(|_, status| {
                    let bg = match status {
                        button::Status::Hovered => color!(0x313244),
                        _ => color!(0x181825),
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: color!(0xcdd6f4),
                        border: iced::Border::default(),
                        ..Default::default()
                    }
                })
                .on_press(Message::HistoryEntryClicked(idx))
                .into()
            })
            .collect();

        let history_list = scrollable(Column::from_vec(history_items).spacing(2)).height(Fill);

        container(column![title, history_list].spacing(12).padding(12))
            .width(180)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(color!(0x181825).into()),
                ..Default::default()
            })
            .into()
    }

    fn view_url_bar(&self) -> Element<Message> {
        let method_picker = pick_list(Method::ALL, Some(self.method), Message::MethodSelected)
            .text_size(13)
            .padding(10)
            .width(100)
            .style(|_, _| pick_list::Style {
                text_color: color!(0x89b4fa),
                placeholder_color: color!(0x6c7086),
                handle_color: color!(0x89b4fa),
                background: color!(0x1e1e2e).into(),
                border: iced::Border {
                    color: color!(0x45475a),
                    width: 1.0,
                    radius: 4.0.into(),
                },
            });

        let url_input = text_input("Enter URL...", &self.url)
            .on_input(Message::UrlChanged)
            .on_submit(Message::Send)
            .padding(10)
            .size(13)
            .width(Fill)
            .style(|_, _| text_input::Style {
                background: color!(0x1e1e2e).into(),
                border: iced::Border {
                    color: color!(0x45475a),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                icon: color!(0x6c7086),
                placeholder: color!(0x6c7086),
                value: color!(0xa6e3a1),
                selection: color!(0x45475a),
            });

        let send_text = if self.loading { "..." } else { "Send" };
        let send_button = button(text(send_text).size(13))
            .padding([10, 24])
            .style(|_, status| {
                let bg = match status {
                    button::Status::Hovered => color!(0x74c7ec),
                    button::Status::Pressed => color!(0x89dceb),
                    _ => color!(0x89b4fa),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: color!(0x1e1e2e),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .on_press_maybe(if self.loading { None } else { Some(Message::Send) });

        let bar = row![method_picker, url_input, send_button]
            .spacing(8)
            .padding(16);

        container(bar)
            .width(Fill)
            .style(|_| container::Style {
                background: Some(color!(0x1e1e2e).into()),
                ..Default::default()
            })
            .into()
    }

    fn view_request_panel(&self) -> Element<Message> {
        let body_active = self.request_tab == RequestTab::Body;
        let headers_active = self.request_tab == RequestTab::Headers;

        let body_tab = button(text("Body").size(12))
            .padding([8, 16])
            .style(move |_, _| {
                let (bg, text_color) = if body_active {
                    (color!(0x1e1e2e), color!(0xcdd6f4))
                } else {
                    (color!(0x181825), color!(0x6c7086))
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color,
                    border: iced::Border::default(),
                    ..Default::default()
                }
            })
            .on_press(Message::RequestTabSelected(RequestTab::Body));

        let headers_tab = button(text("Headers").size(12))
            .padding([8, 16])
            .style(move |_, _| {
                let (bg, text_color) = if headers_active {
                    (color!(0x1e1e2e), color!(0xcdd6f4))
                } else {
                    (color!(0x181825), color!(0x6c7086))
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color,
                    border: iced::Border::default(),
                    ..Default::default()
                }
            })
            .on_press(Message::RequestTabSelected(RequestTab::Headers));

        let tabs = row![body_tab, headers_tab].spacing(0);

        let content: Element<Message> = match self.request_tab {
            RequestTab::Body => text_editor(&self.request_body)
                .placeholder("Request body (JSON, XML, etc.)")
                .on_action(Message::RequestBodyChanged)
                .padding(12)
                .height(Fill)
                .style(|_, _| text_editor::Style {
                    background: color!(0x1e1e2e).into(),
                    border: iced::Border {
                        color: color!(0x45475a),
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    placeholder: color!(0x6c7086),
                    value: color!(0xcdd6f4),
                    selection: color!(0x45475a),
                })
                .into(),
            RequestTab::Headers => text_input("Header: Value", &self.request_headers)
                .on_input(Message::RequestHeadersChanged)
                .padding(12)
                .size(13)
                .width(Fill)
                .style(|_, _| text_input::Style {
                    background: color!(0x1e1e2e).into(),
                    border: iced::Border {
                        color: color!(0x45475a),
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    icon: color!(0x6c7086),
                    placeholder: color!(0x6c7086),
                    value: color!(0xcdd6f4),
                    selection: color!(0x45475a),
                })
                .into(),
        };

        let panel = column![
            container(text("Request").size(11).color(color!(0x6c7086)))
                .padding([12, 16]),
            tabs,
            container(content).padding(12).height(Fill),
        ]
        .spacing(0);

        container(panel)
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(color!(0x181825).into()),
                ..Default::default()
            })
            .into()
    }

    fn view_response_panel(&self) -> Element<Message> {
        let body_active = self.response_tab == ResponseTab::Body;
        let headers_active = self.response_tab == ResponseTab::Headers;

        let body_tab = button(text("Body").size(12))
            .padding([8, 16])
            .style(move |_, _| {
                let (bg, text_color) = if body_active {
                    (color!(0x1e1e2e), color!(0xcdd6f4))
                } else {
                    (color!(0x181825), color!(0x6c7086))
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color,
                    border: iced::Border::default(),
                    ..Default::default()
                }
            })
            .on_press(Message::ResponseTabSelected(ResponseTab::Body));

        let headers_tab = button(text("Headers").size(12))
            .padding([8, 16])
            .style(move |_, _| {
                let (bg, text_color) = if headers_active {
                    (color!(0x1e1e2e), color!(0xcdd6f4))
                } else {
                    (color!(0x181825), color!(0x6c7086))
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color,
                    border: iced::Border::default(),
                    ..Default::default()
                }
            })
            .on_press(Message::ResponseTabSelected(ResponseTab::Headers));

        let tabs = row![body_tab, headers_tab].spacing(0);

        let status_bar: Element<Message> = if let Some(ref response) = self.response {
            let status_color = match response.status {
                200..=299 => color!(0xa6e3a1),
                300..=399 => color!(0x89b4fa),
                400..=499 => color!(0xf9e2af),
                500..=599 => color!(0xf38ba8),
                _ => color!(0xcdd6f4),
            };

            row![
                text(format!("{} {}", response.status, response.status_text))
                    .size(12)
                    .color(status_color),
                text(format!("  {}ms | {}", response.duration.as_millis(), format_size(response.size)))
                    .size(11)
                    .color(color!(0x6c7086)),
            ]
            .spacing(8)
            .into()
        } else if let Some(ref error) = self.error {
            text(format!("Error: {}", error))
                .size(12)
                .color(color!(0xf38ba8))
                .into()
        } else {
            text("No response yet")
                .size(12)
                .color(color!(0x6c7086))
                .into()
        };

        let content: Element<Message> = if let Some(ref response) = self.response {
            match self.response_tab {
                ResponseTab::Body => {
                    let formatted = format_json(&response.body);
                    scrollable(
                        container(text(formatted).size(12).color(color!(0xcdd6f4)))
                            .padding(12)
                            .width(Fill),
                    )
                    .height(Fill)
                    .into()
                }
                ResponseTab::Headers => {
                    let headers_text: String = response
                        .headers
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n");

                    scrollable(
                        container(text(headers_text).size(12).color(color!(0xcdd6f4)))
                            .padding(12)
                            .width(Fill),
                    )
                    .height(Fill)
                    .into()
                }
            }
        } else {
            container(
                text("Send a request to see the response")
                    .size(13)
                    .color(color!(0x6c7086)),
            )
            .padding(16)
            .into()
        };

        let header_row = row![
            text("Response").size(11).color(color!(0x6c7086)),
            status_bar,
        ].spacing(16);

        let panel = column![
            container(header_row).padding([12, 16]),
            tabs,
            container(content).padding(12).height(Fill),
        ]
        .spacing(0);

        container(panel)
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(color!(0x181825).into()),
                ..Default::default()
            })
            .into()
    }
}

async fn send_request(
    url: String,
    method: Method,
    body: String,
    headers_str: String,
) -> Result<Response, String> {
    let start = Instant::now();
    let client = reqwest::Client::new();

    let mut builder = match method {
        Method::GET => client.get(&url),
        Method::POST => client.post(&url),
        Method::PUT => client.put(&url),
        Method::PATCH => client.patch(&url),
        Method::DELETE => client.delete(&url),
        Method::HEAD => client.head(&url),
        Method::OPTIONS => client.request(reqwest::Method::OPTIONS, &url),
    };

    for line in headers_str.lines() {
        if let Some((key, value)) = line.split_once(':') {
            builder = builder.header(key.trim(), value.trim());
        }
    }

    if matches!(method, Method::POST | Method::PUT | Method::PATCH) && !body.is_empty() {
        builder = builder.body(body);
    }

    let response = builder.send().await.map_err(|e| e.to_string())?;
    let duration = start.elapsed();

    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();

    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body = response.text().await.map_err(|e| e.to_string())?;
    let size = body.len();

    Ok(Response {
        status,
        status_text,
        headers,
        body,
        duration,
        size,
    })
}

fn format_json(s: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(s) {
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| s.to_string())
    } else {
        s.to_string()
    }
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
