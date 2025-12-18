use iced::widget::{
    button, column, container, mouse_area, pick_list, row, scrollable, text, text_editor,
    text_input, Column,
};
use iced::{Element, Fill, Font, Length, Padding, Task, Theme};
use iced::time::{self, Duration, Instant};
use std::time::Instant as StdInstant;

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title("BadGateway")
        .theme(App::theme)
        .default_font(Font::MONOSPACE)
        .subscription(App::subscription)
        .run()
}

// Dark industrial palette inspired by the dashboard
mod colors {
    use iced::Color;

    pub const BG_DARKEST: Color = Color::from_rgb(0.08, 0.08, 0.10);      // #14141a
    pub const BG_DARK: Color = Color::from_rgb(0.11, 0.11, 0.13);         // #1c1c21
    pub const BG_PANEL: Color = Color::from_rgb(0.14, 0.14, 0.16);        // #242428
    pub const BG_ELEVATED: Color = Color::from_rgb(0.18, 0.18, 0.20);     // #2e2e33
    pub const BORDER: Color = Color::from_rgb(0.22, 0.22, 0.25);          // #383840
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.85, 0.85, 0.88);    // #d9d9e0
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.50, 0.50, 0.55);  // #80808c
    pub const ACCENT_PURPLE: Color = Color::from_rgb(0.65, 0.45, 0.85);   // #a673d9
    pub const ACCENT_CORAL: Color = Color::from_rgb(0.95, 0.55, 0.45);    // #f28c73
    pub const SUCCESS: Color = Color::from_rgb(0.40, 0.75, 0.55);         // #66bf8c
    pub const WARNING: Color = Color::from_rgb(0.95, 0.75, 0.35);         // #f2bf59
    pub const ERROR: Color = Color::from_rgb(0.95, 0.45, 0.45);           // #f27373
}

fn theme_palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: colors::BG_DARKEST,
        text: colors::TEXT_PRIMARY,
        primary: colors::ACCENT_PURPLE,
        success: colors::SUCCESS,
        warning: colors::WARNING,
        danger: colors::ERROR,
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
        Method::GET, Method::POST, Method::PUT, Method::PATCH,
        Method::DELETE, Method::HEAD, Method::OPTIONS,
    ];

    fn color(&self) -> iced::Color {
        match self {
            Method::GET => colors::SUCCESS,
            Method::POST => colors::ACCENT_CORAL,
            Method::PUT => colors::WARNING,
            Method::PATCH => colors::ACCENT_PURPLE,
            Method::DELETE => colors::ERROR,
            _ => colors::TEXT_SECONDARY,
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::PATCH => "PATCH",
            Method::DELETE => "DELETE",
            Method::HEAD => "HEAD",
            Method::OPTIONS => "OPTIONS",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab { #[default] Body, Headers }

#[derive(Debug, Clone)]
struct Response {
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    body: String,
    duration: std::time::Duration,
    size: usize,
}

struct App {
    url: String,
    method: Method,
    request_tab: Tab,
    response_tab: Tab,
    request_body: text_editor::Content,
    request_headers: String,
    response: Option<Response>,
    loading: bool,
    error: Option<String>,
    history: Vec<HistoryEntry>,
    // Panel sizing
    sidebar_width: f32,
    request_width: f32,
    dragging: Option<DragTarget>,
    // Animation
    last_tick: Option<Instant>,
    sidebar_width_target: f32,
    request_width_target: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragTarget {
    Sidebar,
    RequestPanel,
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
    RequestTabSelected(Tab),
    ResponseTabSelected(Tab),
    RequestBodyChanged(text_editor::Action),
    RequestHeadersChanged(String),
    Send,
    ResponseReceived(Result<Response, String>),
    HistoryEntryClicked(usize),
    // Resizing
    StartDrag(DragTarget),
    Drag(f32),
    EndDrag,
    // Animation
    Tick(Instant),
}

impl Default for App {
    fn default() -> Self {
        Self {
            url: String::from("https://httpbin.org/get"),
            method: Method::GET,
            request_tab: Tab::Body,
            response_tab: Tab::Body,
            request_body: text_editor::Content::new(),
            request_headers: String::from("Content-Type: application/json"),
            response: None,
            loading: false,
            error: None,
            history: Vec::new(),
            sidebar_width: 200.0,
            request_width: 0.5, // 50% of remaining space
            dragging: None,
            last_tick: None,
            sidebar_width_target: 200.0,
            request_width_target: 0.5,
        }
    }
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn theme(&self) -> Theme {
        Theme::custom("Dashboard", theme_palette())
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        // Animate panel sizes smoothly
        let needs_animation = (self.sidebar_width - self.sidebar_width_target).abs() > 0.5
            || (self.request_width - self.request_width_target).abs() > 0.001;

        if needs_animation || self.dragging.is_some() {
            time::every(Duration::from_millis(16)).map(Message::Tick)
        } else {
            iced::Subscription::none()
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UrlChanged(url) => { self.url = url; }
            Message::MethodSelected(method) => { self.method = method; }
            Message::RequestTabSelected(tab) => { self.request_tab = tab; }
            Message::ResponseTabSelected(tab) => { self.response_tab = tab; }
            Message::RequestBodyChanged(action) => { self.request_body.perform(action); }
            Message::RequestHeadersChanged(headers) => { self.request_headers = headers; }
            Message::Send => {
                self.loading = true;
                self.error = None;
                let url = self.url.clone();
                let method = self.method;
                let body = self.request_body.text();
                let headers = self.request_headers.clone();
                return Task::perform(
                    async move { send_request(url, method, body, headers).await },
                    Message::ResponseReceived,
                );
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
            }
            Message::HistoryEntryClicked(index) => {
                if let Some(entry) = self.history.get(index) {
                    self.url = entry.url.clone();
                    self.method = entry.method;
                }
            }
            Message::StartDrag(target) => {
                self.dragging = Some(target);
            }
            Message::Drag(delta) => {
                if let Some(target) = self.dragging {
                    match target {
                        DragTarget::Sidebar => {
                            self.sidebar_width_target = (self.sidebar_width_target + delta).clamp(120.0, 400.0);
                            self.sidebar_width = self.sidebar_width_target;
                        }
                        DragTarget::RequestPanel => {
                            let delta_ratio = delta / 800.0; // approximate
                            self.request_width_target = (self.request_width_target + delta_ratio).clamp(0.25, 0.75);
                            self.request_width = self.request_width_target;
                        }
                    }
                }
            }
            Message::EndDrag => {
                self.dragging = None;
            }
            Message::Tick(_now) => {
                // Smooth animation with easing
                let ease = 0.15;
                self.sidebar_width += (self.sidebar_width_target - self.sidebar_width) * ease;
                self.request_width += (self.request_width_target - self.request_width) * ease;
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let url_bar = self.view_url_bar();

        let sidebar = self.view_sidebar();
        let sidebar_handle = self.view_resize_handle(DragTarget::Sidebar);

        let request_panel = self.view_request_panel();
        let panel_handle = self.view_resize_handle(DragTarget::RequestPanel);
        let response_panel = self.view_response_panel();

        let main_content = row![request_panel, panel_handle, response_panel]
            .height(Fill);

        let content = column![url_bar, main_content].spacing(1).width(Fill);

        let main_view = row![
            container(sidebar).width(Length::Fixed(self.sidebar_width)),
            sidebar_handle,
            content
        ]
        .height(Fill);

        container(main_view)
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(colors::BG_DARKEST.into()),
                ..Default::default()
            })
            .into()
    }

    fn view_resize_handle(&self, target: DragTarget) -> Element<Message> {
        let is_dragging = self.dragging == Some(target);
        let handle_color = if is_dragging { colors::ACCENT_PURPLE } else { colors::BORDER };

        mouse_area(
            container(column![])
                .width(4)
                .height(Fill)
                .style(move |_| container::Style {
                    background: Some(handle_color.into()),
                    ..Default::default()
                })
        )
        .on_press(Message::StartDrag(target))
        .on_release(Message::EndDrag)
        .into()
    }

    fn view_sidebar(&self) -> Element<Message> {
        let title = text("HISTORY").size(10).color(colors::TEXT_SECONDARY);

        let history_content: Element<Message> = if self.history.is_empty() {
            container(
                text("No requests yet").size(11).color(colors::TEXT_SECONDARY)
            )
            .padding(8)
            .into()
        } else {
            let items: Vec<Element<Message>> = self
                .history
                .iter()
                .rev()
                .enumerate()
                .take(50)
                .map(|(i, entry)| {
                    let status_color = match entry.status {
                        200..=299 => colors::SUCCESS,
                        400..=499 => colors::WARNING,
                        500..=599 => colors::ERROR,
                        _ => colors::TEXT_PRIMARY,
                    };

                    let url_display = if entry.url.len() > 18 {
                        format!("{}...", &entry.url[..15])
                    } else {
                        entry.url.clone()
                    };

                    let idx = self.history.len() - 1 - i;

                    button(
                        column![
                            row![
                                text(entry.method.to_string())
                                    .size(10)
                                    .color(entry.method.color()),
                                text(entry.status.to_string())
                                    .size(10)
                                    .color(status_color),
                            ].spacing(8),
                            text(url_display).size(10).color(colors::TEXT_SECONDARY),
                        ].spacing(2),
                    )
                    .width(Fill)
                    .padding(8)
                    .style(|_, status| {
                        let bg = match status {
                            button::Status::Hovered => colors::BG_ELEVATED,
                            _ => colors::BG_PANEL,
                        };
                        button::Style {
                            background: Some(bg.into()),
                            text_color: colors::TEXT_PRIMARY,
                            border: iced::Border::default(),
                            ..Default::default()
                        }
                    })
                    .on_press(Message::HistoryEntryClicked(idx))
                    .into()
                })
                .collect();

            scrollable(Column::from_vec(items).spacing(4).width(Fill))
                .height(Fill)
                .into()
        };

        container(
            column![title, history_content].spacing(12).width(Fill)
        )
        .padding(12)
        .width(Fill)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(colors::BG_DARK.into()),
            ..Default::default()
        })
        .into()
    }

    fn view_url_bar(&self) -> Element<Message> {
        let method_picker = pick_list(Method::ALL, Some(self.method), Message::MethodSelected)
            .text_size(12)
            .padding(10)
            .width(90)
            .style(|_, _| pick_list::Style {
                text_color: self.method.color(),
                placeholder_color: colors::TEXT_SECONDARY,
                handle_color: colors::TEXT_SECONDARY,
                background: colors::BG_ELEVATED.into(),
                border: iced::Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
            });

        let url_input = text_input("https://api.example.com/endpoint", &self.url)
            .on_input(Message::UrlChanged)
            .on_submit(Message::Send)
            .padding(10)
            .size(12)
            .width(Fill)
            .style(|_, _| text_input::Style {
                background: colors::BG_ELEVATED.into(),
                border: iced::Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                icon: colors::TEXT_SECONDARY,
                placeholder: colors::TEXT_SECONDARY,
                value: colors::TEXT_PRIMARY,
                selection: colors::ACCENT_PURPLE,
            });

        let send_text = if self.loading { "..." } else { "SEND" };
        let send_button = button(text(send_text).size(11))
            .padding([10, 20])
            .style(|_, status| {
                let bg = match status {
                    button::Status::Hovered => colors::ACCENT_CORAL,
                    button::Status::Pressed => colors::WARNING,
                    _ => colors::ACCENT_PURPLE,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: colors::BG_DARKEST,
                    border: iced::Border {
                        radius: 0.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .on_press_maybe(if self.loading { None } else { Some(Message::Send) });

        let bar = row![method_picker, url_input, send_button]
            .spacing(8)
            .padding(12);

        container(bar)
            .width(Fill)
            .style(|_| container::Style {
                background: Some(colors::BG_PANEL.into()),
                border: iced::Border {
                    color: colors::BORDER,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_request_panel(&self) -> Element<Message> {
        let body_active = self.request_tab == Tab::Body;
        let headers_active = self.request_tab == Tab::Headers;

        let body_tab = button(text("Body").size(11))
            .padding([10, 16])
            .style(move |_, _| {
                let (bg, txt, border) = if body_active {
                    (colors::BG_PANEL, colors::TEXT_PRIMARY, colors::ACCENT_PURPLE)
                } else {
                    (colors::BG_DARK, colors::TEXT_SECONDARY, colors::BG_DARK)
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: txt,
                    border: iced::Border { color: border, width: if body_active { 2.0 } else { 0.0 }, radius: 0.0.into() },
                    ..Default::default()
                }
            })
            .on_press(Message::RequestTabSelected(Tab::Body));

        let headers_tab = button(text("Headers").size(11))
            .padding([10, 16])
            .style(move |_, _| {
                let (bg, txt, border) = if headers_active {
                    (colors::BG_PANEL, colors::TEXT_PRIMARY, colors::ACCENT_PURPLE)
                } else {
                    (colors::BG_DARK, colors::TEXT_SECONDARY, colors::BG_DARK)
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: txt,
                    border: iced::Border { color: border, width: if headers_active { 2.0 } else { 0.0 }, radius: 0.0.into() },
                    ..Default::default()
                }
            })
            .on_press(Message::RequestTabSelected(Tab::Headers));

        let tabs = row![body_tab, headers_tab].spacing(0);

        let content: Element<Message> = match self.request_tab {
            Tab::Body => text_editor(&self.request_body)
                .placeholder("{\n  \"key\": \"value\"\n}")
                .on_action(Message::RequestBodyChanged)
                .padding(12)
                .height(Fill)
                .style(|_, _| text_editor::Style {
                    background: colors::BG_PANEL.into(),
                    border: iced::Border::default(),
                    placeholder: colors::TEXT_SECONDARY,
                    value: colors::TEXT_PRIMARY,
                    selection: colors::ACCENT_PURPLE,
                })
                .into(),
            Tab::Headers => text_input("Content-Type: application/json", &self.request_headers)
                .on_input(Message::RequestHeadersChanged)
                .padding(12)
                .size(12)
                .width(Fill)
                .style(|_, _| text_input::Style {
                    background: colors::BG_PANEL.into(),
                    border: iced::Border::default(),
                    icon: colors::TEXT_SECONDARY,
                    placeholder: colors::TEXT_SECONDARY,
                    value: colors::TEXT_PRIMARY,
                    selection: colors::ACCENT_PURPLE,
                })
                .into(),
        };

        let header = row![
            text("REQUEST").size(10).color(colors::TEXT_SECONDARY),
        ];

        let panel = column![
            container(header).padding(Padding { top: 12.0, right: 16.0, bottom: 8.0, left: 16.0 }),
            container(tabs).style(|_| container::Style {
                background: Some(colors::BG_DARK.into()),
                ..Default::default()
            }),
            container(content).padding(0).height(Fill),
        ].spacing(0);

        container(panel)
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(colors::BG_PANEL.into()),
                ..Default::default()
            })
            .into()
    }

    fn view_response_panel(&self) -> Element<Message> {
        let body_active = self.response_tab == Tab::Body;
        let headers_active = self.response_tab == Tab::Headers;

        let body_tab = button(text("Body").size(11))
            .padding([10, 16])
            .style(move |_, _| {
                let (bg, txt, border) = if body_active {
                    (colors::BG_PANEL, colors::TEXT_PRIMARY, colors::ACCENT_CORAL)
                } else {
                    (colors::BG_DARK, colors::TEXT_SECONDARY, colors::BG_DARK)
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: txt,
                    border: iced::Border { color: border, width: if body_active { 2.0 } else { 0.0 }, radius: 0.0.into() },
                    ..Default::default()
                }
            })
            .on_press(Message::ResponseTabSelected(Tab::Body));

        let headers_tab = button(text("Headers").size(11))
            .padding([10, 16])
            .style(move |_, _| {
                let (bg, txt, border) = if headers_active {
                    (colors::BG_PANEL, colors::TEXT_PRIMARY, colors::ACCENT_CORAL)
                } else {
                    (colors::BG_DARK, colors::TEXT_SECONDARY, colors::BG_DARK)
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: txt,
                    border: iced::Border { color: border, width: if headers_active { 2.0 } else { 0.0 }, radius: 0.0.into() },
                    ..Default::default()
                }
            })
            .on_press(Message::ResponseTabSelected(Tab::Headers));

        let tabs = row![body_tab, headers_tab].spacing(0);

        let status_bar: Element<Message> = if let Some(ref response) = self.response {
            let status_color = match response.status {
                200..=299 => colors::SUCCESS,
                300..=399 => colors::ACCENT_PURPLE,
                400..=499 => colors::WARNING,
                500..=599 => colors::ERROR,
                _ => colors::TEXT_PRIMARY,
            };

            row![
                text(format!("{}", response.status))
                    .size(11)
                    .color(status_color),
                text(format!("{}", response.status_text))
                    .size(11)
                    .color(colors::TEXT_SECONDARY),
                text(format!("{}ms", response.duration.as_millis()))
                    .size(10)
                    .color(colors::TEXT_SECONDARY),
                text(format_size(response.size))
                    .size(10)
                    .color(colors::TEXT_SECONDARY),
            ]
            .spacing(12)
            .into()
        } else if let Some(ref error) = self.error {
            text(format!("Error: {}", truncate_str(error, 40)))
                .size(11)
                .color(colors::ERROR)
                .into()
        } else {
            text("Ready")
                .size(11)
                .color(colors::TEXT_SECONDARY)
                .into()
        };

        let content: Element<Message> = if let Some(ref response) = self.response {
            match self.response_tab {
                Tab::Body => {
                    let formatted = format_json(&response.body);
                    scrollable(
                        container(text(formatted).size(11).color(colors::TEXT_PRIMARY))
                            .padding(12)
                            .width(Fill),
                    )
                    .height(Fill)
                    .into()
                }
                Tab::Headers => {
                    let headers_text: String = response
                        .headers
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n");

                    scrollable(
                        container(text(headers_text).size(11).color(colors::TEXT_PRIMARY))
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
                    .size(12)
                    .color(colors::TEXT_SECONDARY),
            )
            .padding(16)
            .center_x(Fill)
            .center_y(Fill)
            .into()
        };

        let header = row![
            text("RESPONSE").size(10).color(colors::TEXT_SECONDARY),
            status_bar,
        ].spacing(16);

        let panel = column![
            container(header).padding(Padding { top: 12.0, right: 16.0, bottom: 8.0, left: 16.0 }),
            container(tabs).style(|_| container::Style {
                background: Some(colors::BG_DARK.into()),
                ..Default::default()
            }),
            container(content).padding(0).height(Fill),
        ].spacing(0);

        container(panel)
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(colors::BG_PANEL.into()),
                ..Default::default()
            })
            .into()
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

async fn send_request(
    url: String,
    method: Method,
    body: String,
    headers_str: String,
) -> Result<Response, String> {
    let start = StdInstant::now();
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
