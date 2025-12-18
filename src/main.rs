use iced::widget::{
    button, column, container, mouse_area, pick_list, row, scrollable, text, text_editor,
    text_input, rich_text, span, Column,
};
use iced::keyboard::{self, key, Key};
use iced::event::{self, Event};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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
enum Tab { #[default] Body, Headers, Params, Auth, Timing }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum AuthType {
    #[default]
    None,
    Bearer,
    Basic,
}

impl AuthType {
    const ALL: &'static [AuthType] = &[AuthType::None, AuthType::Bearer, AuthType::Basic];
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            AuthType::None => "No Auth",
            AuthType::Bearer => "Bearer Token",
            AuthType::Basic => "Basic Auth",
        })
    }
}

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
    request_headers: text_editor::Content,
    query_params: text_editor::Content,
    response: Option<Response>,
    loading: bool,
    error: Option<String>,
    history: Vec<HistoryEntry>,
    // Auth
    auth_type: AuthType,
    auth_token: String,
    auth_username: String,
    auth_password: String,
    // cURL import
    show_curl_import: bool,
    curl_input: String,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    RequestHeadersChanged(text_editor::Action),
    QueryParamsChanged(text_editor::Action),
    Send,
    ResponseReceived(Result<Response, String>),
    HistoryEntryClicked(usize),
    CopyResponse,
    // Auth
    AuthTypeSelected(AuthType),
    AuthTokenChanged(String),
    AuthUsernameChanged(String),
    AuthPasswordChanged(String),
    // cURL import
    ToggleCurlImport,
    CurlInputChanged(String),
    ImportCurl,
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
            request_headers: text_editor::Content::with_text("Content-Type: application/json\n"),
            query_params: text_editor::Content::new(),
            response: None,
            loading: false,
            error: None,
            history: load_history(),
            auth_type: AuthType::None,
            auth_token: String::new(),
            auth_username: String::new(),
            auth_password: String::new(),
            show_curl_import: false,
            curl_input: String::new(),
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
        let keyboard_sub = event::listen_with(|event, _status, _id| {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                // Cmd/Ctrl + Enter to send
                if key == Key::Named(key::Named::Enter) && modifiers.command() {
                    return Some(Message::Send);
                }
            }
            None
        });

        // Animate panel sizes smoothly
        let needs_animation = (self.sidebar_width - self.sidebar_width_target).abs() > 0.5
            || (self.request_width - self.request_width_target).abs() > 0.001;

        if needs_animation || self.dragging.is_some() {
            iced::Subscription::batch([
                keyboard_sub,
                time::every(Duration::from_millis(16)).map(Message::Tick),
            ])
        } else {
            keyboard_sub
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UrlChanged(url) => { self.url = url; }
            Message::MethodSelected(method) => { self.method = method; }
            Message::RequestTabSelected(tab) => { self.request_tab = tab; }
            Message::ResponseTabSelected(tab) => { self.response_tab = tab; }
            Message::RequestBodyChanged(action) => { self.request_body.perform(action); }
            Message::RequestHeadersChanged(action) => { self.request_headers.perform(action); }
            Message::QueryParamsChanged(action) => { self.query_params.perform(action); }
            Message::AuthTypeSelected(auth_type) => { self.auth_type = auth_type; }
            Message::AuthTokenChanged(token) => { self.auth_token = token; }
            Message::AuthUsernameChanged(username) => { self.auth_username = username; }
            Message::AuthPasswordChanged(password) => { self.auth_password = password; }
            Message::ToggleCurlImport => { self.show_curl_import = !self.show_curl_import; }
            Message::CurlInputChanged(input) => { self.curl_input = input; }
            Message::ImportCurl => {
                if let Some(parsed) = parse_curl(&self.curl_input) {
                    self.url = parsed.url;
                    self.method = parsed.method;
                    if !parsed.headers.is_empty() {
                        self.request_headers = text_editor::Content::with_text(&parsed.headers);
                    }
                    if !parsed.body.is_empty() {
                        self.request_body = text_editor::Content::with_text(&parsed.body);
                    }
                    if let Some((auth_type, token, user, pass)) = parsed.auth {
                        self.auth_type = auth_type;
                        self.auth_token = token;
                        self.auth_username = user;
                        self.auth_password = pass;
                    }
                }
                self.show_curl_import = false;
                self.curl_input.clear();
            }
            Message::Send => {
                self.loading = true;
                self.error = None;
                // Build URL with query params
                let mut url = self.url.clone();
                let params = self.query_params.text();
                if !params.trim().is_empty() {
                    let param_pairs: Vec<&str> = params.lines()
                        .filter(|l| !l.trim().is_empty() && l.contains('='))
                        .collect();
                    if !param_pairs.is_empty() {
                        let separator = if url.contains('?') { "&" } else { "?" };
                        url.push_str(separator);
                        url.push_str(&param_pairs.join("&"));
                    }
                }
                let method = self.method;
                let body = self.request_body.text();
                let headers = self.request_headers.text();
                let auth_type = self.auth_type;
                let auth_token = self.auth_token.clone();
                let auth_username = self.auth_username.clone();
                let auth_password = self.auth_password.clone();
                return Task::perform(
                    async move {
                        send_request(url, method, body, headers, auth_type, auth_token, auth_username, auth_password).await
                    },
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
                        save_history(&self.history);
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
            Message::CopyResponse => {
                if let Some(ref response) = self.response {
                    let text = match self.response_tab {
                        Tab::Body | Tab::Params | Tab::Auth => format_json(&response.body),
                        Tab::Headers => response.headers.iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        Tab::Timing => format!(
                            "Total Time: {}ms\nResponse Size: {}\nTransfer Speed: {:.1} KB/s",
                            response.duration.as_millis(),
                            format_size(response.size),
                            if response.duration.as_secs_f64() > 0.0 {
                                (response.size as f64 / response.duration.as_secs_f64()) / 1024.0
                            } else { 0.0 }
                        ),
                    };
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(text);
                    }
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

        let status_bar = self.view_status_bar();

        let with_status = column![main_view, status_bar].height(Fill);

        let base: Element<Message> = container(with_status)
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(colors::BG_DARKEST.into()),
                ..Default::default()
            })
            .into();

        // Show curl import modal if needed
        if self.show_curl_import {
            use iced::widget::stack;

            let modal_overlay = container(column![])
                .width(Fill)
                .height(Fill)
                .style(|_| container::Style {
                    background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
                    ..Default::default()
                });

            let curl_input = text_input("Paste cURL command here...", &self.curl_input)
                .on_input(Message::CurlInputChanged)
                .on_submit(Message::ImportCurl)
                .padding(12)
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

            let import_btn = button(text("IMPORT").size(11))
                .padding([10, 20])
                .style(|_, status| {
                    let bg = match status {
                        button::Status::Hovered => colors::ACCENT_CORAL,
                        _ => colors::ACCENT_PURPLE,
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: colors::BG_DARKEST,
                        border: iced::Border { radius: 0.0.into(), ..Default::default() },
                        ..Default::default()
                    }
                })
                .on_press(Message::ImportCurl);

            let cancel_btn = button(text("CANCEL").size(11))
                .padding([10, 20])
                .style(|_, status| {
                    let bg = match status {
                        button::Status::Hovered => colors::BG_ELEVATED,
                        _ => colors::BG_DARK,
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: colors::TEXT_PRIMARY,
                        border: iced::Border {
                            color: colors::BORDER,
                            width: 1.0,
                            radius: 0.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::ToggleCurlImport);

            let modal_content = container(
                column![
                    text("IMPORT CURL").size(12).color(colors::TEXT_SECONDARY),
                    curl_input,
                    text("Paste a cURL command and press Enter or click Import")
                        .size(10)
                        .color(colors::TEXT_SECONDARY),
                    row![cancel_btn, import_btn].spacing(8),
                ]
                .spacing(12)
                .width(Length::Fixed(500.0))
            )
            .padding(20)
            .style(|_| container::Style {
                background: Some(colors::BG_PANEL.into()),
                border: iced::Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            });

            let modal_centered = container(modal_content)
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill);

            stack![base, modal_overlay, modal_centered].into()
        } else {
            base
        }
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

    fn view_status_bar(&self) -> Element<Message> {
        let method_color = self.method.color();

        let left_items = row![
            text(self.method.to_string()).size(10).color(method_color),
            text(truncate_str(&self.url, 50)).size(10).color(colors::TEXT_SECONDARY),
        ]
        .spacing(8);

        let auth_indicator = match self.auth_type {
            AuthType::None => text("").size(10),
            AuthType::Bearer => text("Bearer").size(10).color(colors::SUCCESS),
            AuthType::Basic => text("Basic").size(10).color(colors::SUCCESS),
        };

        let status_indicator = if self.loading {
            text("Sending...").size(10).color(colors::WARNING)
        } else if self.response.is_some() {
            text("Ready").size(10).color(colors::SUCCESS)
        } else if self.error.is_some() {
            text("Error").size(10).color(colors::ERROR)
        } else {
            text("Ready").size(10).color(colors::TEXT_SECONDARY)
        };

        let history_count = text(format!("{} requests", self.history.len()))
            .size(10)
            .color(colors::TEXT_SECONDARY);

        let shortcut_hint = text("Cmd+Enter to send")
            .size(10)
            .color(colors::TEXT_SECONDARY);

        let right_items = row![
            auth_indicator,
            status_indicator,
            history_count,
            shortcut_hint,
        ]
        .spacing(16);

        container(
            row![left_items, right_items]
                .width(Fill)
                .align_y(iced::Alignment::Center)
        )
        .padding(Padding { top: 6.0, right: 16.0, bottom: 6.0, left: 16.0 })
        .width(Fill)
        .style(|_| container::Style {
            background: Some(colors::BG_DARK.into()),
            border: iced::Border {
                color: colors::BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
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

        let import_button = button(text("cURL").size(10))
            .padding([10, 12])
            .style(|_, status| {
                let bg = match status {
                    button::Status::Hovered => colors::BG_ELEVATED,
                    _ => colors::BG_DARK,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: colors::TEXT_SECONDARY,
                    border: iced::Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::ToggleCurlImport);

        let bar = row![method_picker, url_input, import_button, send_button]
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
        let params_active = self.request_tab == Tab::Params;
        let auth_active = self.request_tab == Tab::Auth;

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

        let params_tab = button(text("Params").size(11))
            .padding([10, 16])
            .style(move |_, _| {
                let (bg, txt, border) = if params_active {
                    (colors::BG_PANEL, colors::TEXT_PRIMARY, colors::ACCENT_PURPLE)
                } else {
                    (colors::BG_DARK, colors::TEXT_SECONDARY, colors::BG_DARK)
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: txt,
                    border: iced::Border { color: border, width: if params_active { 2.0 } else { 0.0 }, radius: 0.0.into() },
                    ..Default::default()
                }
            })
            .on_press(Message::RequestTabSelected(Tab::Params));

        let auth_tab = button(text("Auth").size(11))
            .padding([10, 16])
            .style(move |_, _| {
                let (bg, txt, border) = if auth_active {
                    (colors::BG_PANEL, colors::TEXT_PRIMARY, colors::ACCENT_PURPLE)
                } else {
                    (colors::BG_DARK, colors::TEXT_SECONDARY, colors::BG_DARK)
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: txt,
                    border: iced::Border { color: border, width: if auth_active { 2.0 } else { 0.0 }, radius: 0.0.into() },
                    ..Default::default()
                }
            })
            .on_press(Message::RequestTabSelected(Tab::Auth));

        let tabs = row![body_tab, headers_tab, params_tab, auth_tab].spacing(0);

        let content: Element<Message> = match self.request_tab {
            Tab::Body | Tab::Timing => text_editor(&self.request_body)
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
            Tab::Headers => text_editor(&self.request_headers)
                .placeholder("Content-Type: application/json\nAuthorization: Bearer token")
                .on_action(Message::RequestHeadersChanged)
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
            Tab::Params => text_editor(&self.query_params)
                .placeholder("key=value\npage=1\nlimit=10")
                .on_action(Message::QueryParamsChanged)
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
            Tab::Auth => self.view_auth_panel(),
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

    fn view_auth_panel(&self) -> Element<Message> {
        let auth_picker = pick_list(AuthType::ALL, Some(self.auth_type), Message::AuthTypeSelected)
            .text_size(12)
            .padding(10)
            .width(200)
            .style(|_, _| pick_list::Style {
                text_color: colors::TEXT_PRIMARY,
                placeholder_color: colors::TEXT_SECONDARY,
                handle_color: colors::TEXT_SECONDARY,
                background: colors::BG_ELEVATED.into(),
                border: iced::Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
            });

        let auth_fields: Element<Message> = match self.auth_type {
            AuthType::None => {
                container(
                    text("No authentication will be sent with the request")
                        .size(11)
                        .color(colors::TEXT_SECONDARY)
                )
                .padding(16)
                .into()
            }
            AuthType::Bearer => {
                let token_input = text_input("Enter token", &self.auth_token)
                    .on_input(Message::AuthTokenChanged)
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

                column![
                    text("Token").size(11).color(colors::TEXT_SECONDARY),
                    token_input,
                    text("Will send: Authorization: Bearer <token>")
                        .size(10)
                        .color(colors::TEXT_SECONDARY),
                ]
                .spacing(8)
                .padding(16)
                .into()
            }
            AuthType::Basic => {
                let username_input = text_input("Username", &self.auth_username)
                    .on_input(Message::AuthUsernameChanged)
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

                let password_input = text_input("Password", &self.auth_password)
                    .on_input(Message::AuthPasswordChanged)
                    .padding(10)
                    .size(12)
                    .width(Fill)
                    .secure(true)
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

                column![
                    column![
                        text("Username").size(11).color(colors::TEXT_SECONDARY),
                        username_input,
                    ].spacing(4),
                    column![
                        text("Password").size(11).color(colors::TEXT_SECONDARY),
                        password_input,
                    ].spacing(4),
                    text("Will send: Authorization: Basic <base64>")
                        .size(10)
                        .color(colors::TEXT_SECONDARY),
                ]
                .spacing(12)
                .padding(16)
                .into()
            }
        };

        let content = column![
            container(
                column![
                    text("AUTH TYPE").size(10).color(colors::TEXT_SECONDARY),
                    auth_picker,
                ]
                .spacing(8)
            )
            .padding(16),
            auth_fields,
        ]
        .spacing(0);

        scrollable(content).height(Fill).into()
    }

    fn view_response_panel(&self) -> Element<Message> {
        let body_active = self.response_tab == Tab::Body;
        let headers_active = self.response_tab == Tab::Headers;
        let timing_active = self.response_tab == Tab::Timing;

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

        let timing_tab = button(text("Timing").size(11))
            .padding([10, 16])
            .style(move |_, _| {
                let (bg, txt, border) = if timing_active {
                    (colors::BG_PANEL, colors::TEXT_PRIMARY, colors::ACCENT_CORAL)
                } else {
                    (colors::BG_DARK, colors::TEXT_SECONDARY, colors::BG_DARK)
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: txt,
                    border: iced::Border { color: border, width: if timing_active { 2.0 } else { 0.0 }, radius: 0.0.into() },
                    ..Default::default()
                }
            })
            .on_press(Message::ResponseTabSelected(Tab::Timing));

        let tabs = row![body_tab, headers_tab, timing_tab].spacing(0);

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
                Tab::Body | Tab::Params | Tab::Auth => {
                    let spans = json_to_spans(&response.body);
                    scrollable(
                        container(rich_text(spans).size(11))
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
                Tab::Timing => {
                    self.view_timing_details(response)
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

        let copy_btn = button(text("COPY").size(9))
            .padding([4, 8])
            .style(|_, status| {
                let bg = match status {
                    button::Status::Hovered => colors::BG_ELEVATED,
                    _ => colors::BG_DARK,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: colors::TEXT_SECONDARY,
                    border: iced::Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .on_press_maybe(if self.response.is_some() { Some(Message::CopyResponse) } else { None });

        let header = row![
            text("RESPONSE").size(10).color(colors::TEXT_SECONDARY),
            status_bar,
            copy_btn,
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

    fn view_timing_details(&self, response: &Response) -> Element<Message> {
        let total_ms = response.duration.as_millis() as f32;
        let bar_width = 300.0;

        // Summary stats
        let speed = if response.duration.as_secs_f64() > 0.0 {
            (response.size as f64 / response.duration.as_secs_f64()) / 1024.0
        } else {
            0.0
        };

        let summary_items = column![
            row![
                text("Total Time").size(12).color(colors::TEXT_SECONDARY),
                text(format!("{}ms", response.duration.as_millis()))
                    .size(14)
                    .color(colors::ACCENT_CORAL),
            ].spacing(12),
            row![
                text("Response Size").size(12).color(colors::TEXT_SECONDARY),
                text(format_size(response.size)).size(14).color(colors::SUCCESS),
            ].spacing(12),
            row![
                text("Transfer Speed").size(12).color(colors::TEXT_SECONDARY),
                text(format!("{:.1} KB/s", speed)).size(14).color(colors::ACCENT_PURPLE),
            ].spacing(12),
        ]
        .spacing(8);

        // Visual breakdown bar
        let timing_note = text("Breakdown (total request time)")
            .size(10)
            .color(colors::TEXT_SECONDARY);

        let bar = container(column![])
            .width(Length::Fixed(bar_width))
            .height(16)
            .style(|_| container::Style {
                background: Some(colors::ACCENT_CORAL.into()),
                ..Default::default()
            });

        let bar_bg = container(bar)
            .width(Length::Fixed(bar_width))
            .style(|_| container::Style {
                background: Some(colors::BG_DARK.into()),
                ..Default::default()
            });

        let timing_bar_row = row![
            container(text("Total").size(11).color(colors::TEXT_SECONDARY))
                .width(Length::Fixed(120.0)),
            bar_bg,
            container(text(format!("{:.0}ms", total_ms)).size(11).color(colors::TEXT_PRIMARY))
                .width(Length::Fixed(80.0))
                .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 12.0 }),
        ]
        .spacing(8);

        let content = column![
            container(
                column![
                    text("TIMING SUMMARY").size(10).color(colors::TEXT_SECONDARY),
                    summary_items,
                ]
                .spacing(12)
            )
            .padding(16)
            .width(Fill)
            .style(|_| container::Style {
                background: Some(colors::BG_ELEVATED.into()),
                ..Default::default()
            }),
            container(
                column![
                    timing_note,
                    timing_bar_row,
                ]
                .spacing(12)
            )
            .padding(16)
            .width(Fill),
        ]
        .spacing(16);

        scrollable(content).height(Fill).into()
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
    auth_type: AuthType,
    auth_token: String,
    auth_username: String,
    auth_password: String,
) -> Result<Response, String> {
    use base64::Engine;
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

    // Add auth header
    match auth_type {
        AuthType::None => {}
        AuthType::Bearer => {
            if !auth_token.is_empty() {
                builder = builder.header("Authorization", format!("Bearer {}", auth_token));
            }
        }
        AuthType::Basic => {
            if !auth_username.is_empty() {
                let credentials = format!("{}:{}", auth_username, auth_password);
                let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
                builder = builder.header("Authorization", format!("Basic {}", encoded));
            }
        }
    }

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

fn json_to_spans<'a>(s: &str) -> Vec<iced::widget::text::Span<'a, iced::Font>> {
    let formatted = format_json(s);
    let mut spans = Vec::new();
    let mut chars = formatted.chars().peekable();
    let mut current = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                // Push any pending text
                if !current.is_empty() {
                    spans.push(span(std::mem::take(&mut current)).color(colors::TEXT_PRIMARY));
                }
                // Read the string
                let mut string_content = String::from("\"");
                let mut is_key = false;
                while let Some(c) = chars.next() {
                    string_content.push(c);
                    if c == '"' {
                        break;
                    }
                    if c == '\\' {
                        if let Some(escaped) = chars.next() {
                            string_content.push(escaped);
                        }
                    }
                }
                // Check if this is a key (followed by :)
                let mut temp_chars = chars.clone();
                while let Some(&c) = temp_chars.peek() {
                    if c.is_whitespace() {
                        temp_chars.next();
                    } else {
                        is_key = c == ':';
                        break;
                    }
                }
                let color = if is_key {
                    colors::ACCENT_PURPLE  // Keys in purple
                } else {
                    colors::SUCCESS        // String values in green
                };
                spans.push(span(string_content).color(color));
            }
            c if c.is_ascii_digit() || c == '-' => {
                if !current.is_empty() {
                    spans.push(span(std::mem::take(&mut current)).color(colors::TEXT_PRIMARY));
                }
                let mut num = String::from(c);
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() || next == '.' || next == 'e' || next == 'E' || next == '+' || next == '-' {
                        num.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                spans.push(span(num).color(colors::ACCENT_CORAL));  // Numbers in coral
            }
            't' | 'f' | 'n' => {
                if !current.is_empty() {
                    spans.push(span(std::mem::take(&mut current)).color(colors::TEXT_PRIMARY));
                }
                let mut keyword = String::from(ch);
                let expected = match ch {
                    't' => "rue",
                    'f' => "alse",
                    'n' => "ull",
                    _ => "",
                };
                for expected_char in expected.chars() {
                    if chars.peek() == Some(&expected_char) {
                        keyword.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if keyword == "true" || keyword == "false" || keyword == "null" {
                    spans.push(span(keyword).color(colors::WARNING));  // Booleans/null in yellow
                } else {
                    spans.push(span(keyword).color(colors::TEXT_PRIMARY));
                }
            }
            '{' | '}' | '[' | ']' | ':' | ',' => {
                if !current.is_empty() {
                    spans.push(span(std::mem::take(&mut current)).color(colors::TEXT_PRIMARY));
                }
                spans.push(span(ch.to_string()).color(colors::TEXT_SECONDARY));  // Punctuation dimmed
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        spans.push(span(current).color(colors::TEXT_PRIMARY));
    }

    if spans.is_empty() {
        spans.push(span(s.to_string()).color(colors::TEXT_PRIMARY));
    }

    spans
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

fn history_path() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("badgateway").join("history.json"))
}

fn load_history() -> Vec<HistoryEntry> {
    if let Some(path) = history_path() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(history) = serde_json::from_str(&data) {
                return history;
            }
        }
    }
    Vec::new()
}

fn save_history(history: &[HistoryEntry]) {
    if let Some(path) = history_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(history) {
            let _ = std::fs::write(path, data);
        }
    }
}

struct ParsedCurl {
    url: String,
    method: Method,
    headers: String,
    body: String,
    auth: Option<(AuthType, String, String, String)>, // (type, token, user, pass)
}

fn parse_curl(input: &str) -> Option<ParsedCurl> {
    let input = input.trim();
    if !input.starts_with("curl") {
        return None;
    }

    let mut url = String::new();
    let mut method = Method::GET;
    let mut headers = Vec::new();
    let mut body = String::new();
    let mut auth: Option<(AuthType, String, String, String)> = None;

    // Simple tokenizer that handles quoted strings
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';

    for ch in input.chars() {
        match ch {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
            }
            c if c == quote_char && in_quotes => {
                in_quotes = false;
            }
            ' ' | '\n' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            '\\' if !in_quotes => {
                // Skip backslash line continuations
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token.as_str() {
            "-X" | "--request" => {
                if i + 1 < tokens.len() {
                    method = match tokens[i + 1].to_uppercase().as_str() {
                        "GET" => Method::GET,
                        "POST" => Method::POST,
                        "PUT" => Method::PUT,
                        "PATCH" => Method::PATCH,
                        "DELETE" => Method::DELETE,
                        "HEAD" => Method::HEAD,
                        "OPTIONS" => Method::OPTIONS,
                        _ => Method::GET,
                    };
                    i += 1;
                }
            }
            "-H" | "--header" => {
                if i + 1 < tokens.len() {
                    let header = &tokens[i + 1];
                    // Check for Authorization header
                    if header.to_lowercase().starts_with("authorization:") {
                        let value = header.splitn(2, ':').nth(1).unwrap_or("").trim();
                        if value.to_lowercase().starts_with("bearer ") {
                            auth = Some((
                                AuthType::Bearer,
                                value[7..].to_string(),
                                String::new(),
                                String::new(),
                            ));
                        } else if value.to_lowercase().starts_with("basic ") {
                            // Try to decode basic auth
                            if let Ok(decoded) = base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                value[6..].trim(),
                            ) {
                                if let Ok(creds) = String::from_utf8(decoded) {
                                    if let Some((user, pass)) = creds.split_once(':') {
                                        auth = Some((
                                            AuthType::Basic,
                                            String::new(),
                                            user.to_string(),
                                            pass.to_string(),
                                        ));
                                    }
                                }
                            }
                        } else {
                            headers.push(header.clone());
                        }
                    } else {
                        headers.push(header.clone());
                    }
                    i += 1;
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                if i + 1 < tokens.len() {
                    body = tokens[i + 1].clone();
                    if method == Method::GET {
                        method = Method::POST;
                    }
                    i += 1;
                }
            }
            "-u" | "--user" => {
                if i + 1 < tokens.len() {
                    let creds = &tokens[i + 1];
                    if let Some((user, pass)) = creds.split_once(':') {
                        auth = Some((
                            AuthType::Basic,
                            String::new(),
                            user.to_string(),
                            pass.to_string(),
                        ));
                    }
                    i += 1;
                }
            }
            s if s.starts_with("http://") || s.starts_with("https://") => {
                url = s.to_string();
            }
            _ => {}
        }
        i += 1;
    }

    if url.is_empty() {
        return None;
    }

    Some(ParsedCurl {
        url,
        method,
        headers: headers.join("\n"),
        body,
        auth,
    })
}
