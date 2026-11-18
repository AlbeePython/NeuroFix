use std::net::Ipv4Addr;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, FontFamily, FontId, Layout, RichText, Sense, Stroke, Ui, Vec2,
};

use crate::checks::{Engine, Event};
use crate::config;
use crate::dns::{self, Resolver, SERVICES};
use crate::icons::{self, Icon};
use crate::theme;
use crate::widgets::{self, PillKind};

const VERSION: &str = "1.3.0";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Dns,
    Vpn,
    About,
}

struct ResolverView {
    resolver: Resolver,
    latency: Option<u64>,
    checking_latency: bool,
    services: [Option<bool>; 3],
    checking_services: bool,
}

impl ResolverView {
    fn new(resolver: Resolver) -> Self {
        Self {
            resolver,
            latency: None,
            checking_latency: false,
            services: [None, None, None],
            checking_services: false,
        }
    }
}

enum Action {
    Apply(Resolver),
    Check(Resolver),
    Sweep,
    Reset,
}

pub struct App {
    page: Page,
    views: Vec<ResolverView>,
    search: String,
    engine: Engine,
    rx: Receiver<Event>,
    alias: Option<String>,
    current: Vec<String>,
    applied: Option<usize>,
    sweep_active: bool,
    sweep_left: usize,
    sweep_apply: bool,
    pending_apply: Option<usize>,
    online: bool,
    dns_ok: bool,
    dns_failures: u32,
    auto_recovery: bool,
    last_failover: Option<Instant>,
    toast: Option<(String, Instant)>,
    add_dialog: bool,
    new_name: String,
    new_primary: String,
    new_secondary: String,
    dialog_error: Option<String>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);

        let builtin = dns::builtin();
        let next_id = dns::new_id(&builtin);
        let custom = config::load_custom(next_id);

        let mut views: Vec<ResolverView> = builtin
            .into_iter()
            .chain(custom)
            .map(ResolverView::new)
            .collect();

        let (engine, rx) = Engine::new(cc.egui_ctx.clone());

        for view in &mut views {
            view.checking_latency = true;
            view.checking_services = true;
            engine.measure(view.resolver.clone());
            engine.check_services(view.resolver.clone());
        }
        engine.refresh_config();

        let total = views.len().max(1);
        Self {
            page: Page::Home,
            views,
            search: String::new(),
            engine,
            rx,
            alias: None,
            current: Vec::new(),
            applied: None,
            sweep_active: false,
            sweep_left: total,
            sweep_apply: false,
            pending_apply: None,
            online: true,
            dns_ok: true,
            dns_failures: 0,
            auto_recovery: true,
            last_failover: None,
            toast: None,
            add_dialog: false,
            new_name: String::new(),
            new_primary: String::new(),
            new_secondary: String::new(),
            dialog_error: None,
        }
    }

    fn view_mut(&mut self, id: usize) -> Option<&mut ResolverView> {
        self.views.iter_mut().find(|view| view.resolver.id == id)
    }

    fn toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }

    fn start_sweep(&mut self, apply: bool) {
        self.sweep_active = true;
        self.sweep_apply = apply;
        self.sweep_left = self.views.len();
        for view in &mut self.views {
            view.checking_latency = true;
            self.engine.measure(view.resolver.clone());
        }
    }

    fn finish_sweep(&mut self) {
        if !self.sweep_apply {
            return;
        }
        let best = self
            .views
            .iter()
            .filter_map(|view| view.latency.map(|ms| (ms, view.resolver.clone())))
            .min_by_key(|(ms, _)| *ms);

        match best {
            Some((ms, resolver)) => {
                self.toast(format!("Лучший DNS: {} — {} мс", resolver.name, ms));
                self.pending_apply = Some(resolver.id);
                self.engine.apply(resolver, self.alias.clone());
            }
            None => self.toast("Ни один резолвер не ответил"),
        }
    }

    fn failover(&mut self) {
        let candidate = self
            .views
            .iter()
            .filter(|view| Some(view.resolver.id) != self.applied)
            .filter(|view| view.services.contains(&Some(true)))
            .filter_map(|view| view.latency.map(|ms| (ms, view.resolver.clone())))
            .min_by_key(|(ms, _)| *ms)
            .map(|(_, resolver)| resolver)
            .or_else(|| {
                self.views
                    .iter()
                    .filter(|view| Some(view.resolver.id) != self.applied)
                    .filter_map(|view| view.latency.map(|ms| (ms, view.resolver.clone())))
                    .min_by_key(|(ms, _)| *ms)
                    .map(|(_, resolver)| resolver)
            });

        match candidate {
            Some(resolver) => {
                self.toast(format!("DNS переключён автоматически: {}", resolver.name));
                self.pending_apply = Some(resolver.id);
                self.engine.apply(resolver, self.alias.clone());
            }
            None => {
                self.toast("Резолверы недоступны — DNS отключён");
                self.engine.reset(self.alias.clone());
            }
        }
        self.last_failover = Some(Instant::now());
        self.dns_failures = 0;
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Latency(id, value) => {
                    if let Some(view) = self.view_mut(id) {
                        view.latency = value;
                        view.checking_latency = false;
                    }
                    if self.sweep_active {
                        self.sweep_left = self.sweep_left.saturating_sub(1);
                        if self.sweep_left == 0 {
                            self.sweep_active = false;
                            self.finish_sweep();
                        }
                    }
                }
                Event::Service(id, index, ok) => {
                    if let Some(view) = self.view_mut(id) {
                        if index < view.services.len() {
                            view.services[index] = Some(ok);
                        }
                        if view.services.iter().all(|state| state.is_some()) {
                            view.checking_services = false;
                        }
                    }
                }
                Event::Applied(id, ok, message) => {
                    self.pending_apply = None;
                    if ok {
                        self.applied = id;
                        if id.is_none() {
                            self.dns_failures = 0;
                        }
                        self.engine.refresh_config();
                        self.engine.probe_health();
                    }
                    self.toast(message);
                }
                Event::Health { dns_ok, online } => {
                    self.dns_ok = dns_ok;
                    self.online = online;
                    if !online {
                        self.dns_failures = 0;
                        if self.applied.is_some() && self.auto_recovery && self.failover_ready() {
                            self.toast("Сеть недоступна — DNS отключён");
                            self.engine.reset(self.alias.clone());
                            self.last_failover = Some(Instant::now());
                        }
                    } else if !dns_ok {
                        self.dns_failures += 1;
                        if self.dns_failures >= 2
                            && self.applied.is_some()
                            && self.auto_recovery
                            && self.failover_ready()
                        {
                            self.failover();
                        }
                    } else {
                        self.dns_failures = 0;
                    }
                }
                Event::Config { alias, servers } => {
                    if servers.is_empty() && self.applied.is_some() {
                        self.applied = None;
                    }
                    self.alias = alias;
                    self.current = servers;
                }
            }
        }
    }

    fn failover_ready(&self) -> bool {
        self.last_failover
            .map(|time| time.elapsed() > Duration::from_secs(45))
            .unwrap_or(true)
    }

    fn rail(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
            ui.painter().rect_filled(rect, 14.0, theme::PRIMARY);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "N",
                FontId::new(22.0, theme::bold()),
                theme::ON_PRIMARY,
            );
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.add_space(2.0);
                ui.add(egui::Label::new(theme::heading("NeuroFix", 19.0)).selectable(false));
                ui.add(
                    egui::Label::new(theme::subtle(&format!("версия v{}", VERSION), 12.0))
                        .selectable(false),
                );
            });
        });

        ui.add_space(26.0);

        if widgets::nav_item(ui, Icon::Home, "Главная", self.page == Page::Home, true).clicked()
        {
            self.page = Page::Home;
        }
        ui.add_space(4.0);
        if widgets::nav_item(ui, Icon::Globe, "DNS", self.page == Page::Dns, true).clicked() {
            self.page = Page::Dns;
        }
        ui.add_space(4.0);
        let vpn = widgets::nav_item(ui, Icon::Shield, "VPN", false, false);
        if vpn.clicked() {
            self.page = Page::Vpn;
        }
        ui.add_space(4.0);
        if widgets::nav_item(ui, Icon::Info, "О NeuroFix", self.page == Page::About, true).clicked()
        {
            self.page = Page::About;
        }

        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.add(egui::Label::new(theme::subtle("Apache License 2.0", 11.5)).selectable(false));
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                let color = if self.alias.is_some() && self.online {
                    Color32::from_rgb(0x2E, 0x7D, 0x32)
                } else {
                    theme::ERROR
                };
                widgets::status_dot(ui, color, 14.0);
                ui.add(
                    egui::Label::new(theme::subtle(
                        if self.online && self.alias.is_some() {
                            "Подключение активно"
                        } else {
                            "Нет подключения"
                        },
                        12.5,
                    ))
                    .selectable(false),
                );
            });
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(theme::subtle(
                    &self
                        .alias
                        .clone()
                        .unwrap_or_else(|| "адаптер не найден".to_owned()),
                    11.5,
                ))
                .selectable(false),
            );
        });
    }

    fn header(ui: &mut Ui, title: &str, subtitle: &str) {
        ui.add(egui::Label::new(theme::heading(title, 32.0)).selectable(false));
        ui.add_space(2.0);
        ui.add(egui::Label::new(theme::subtle(subtitle, 14.0)).selectable(false));
    }

    fn home(&mut self, ui: &mut Ui) {
        Self::header(
            ui,
            "Главная",
            "Состояние подключения и сведения о приложении",
        );
        ui.add_space(22.0);

        let width = ui.available_width();
        let gap = 20.0;
        let left_width = ((width - gap) * 0.56).max(320.0);
        let right_width = (width - left_width - gap).max(280.0);

        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(left_width, 0.0),
                Layout::top_down(Align::Min),
                |ui| {
                    widgets::card(ui, theme::SURFACE_LOW, 24.0, |ui| {
                        ui.add(
                            egui::Label::new(theme::heading("Состояние сети", 17.0))
                                .selectable(false),
                        );
                        ui.add_space(14.0);

                        info_row(ui, "Подключение", &self.alias.clone().unwrap_or_else(|| "не найдено".to_owned()));
                        info_row(
                            ui,
                            "Текущий DNS",
                            &if self.current.is_empty() {
                                "Автоматически (DHCP)".to_owned()
                            } else {
                                self.current.join("  ·  ")
                            },
                        );
                        info_row(
                            ui,
                            "Активный профиль",
                            &self
                                .applied
                                .and_then(|id| self.views.iter().find(|view| view.resolver.id == id))
                                .map(|view| view.resolver.name.clone())
                                .unwrap_or_else(|| "не выбран".to_owned()),
                        );
                        info_row(
                            ui,
                            "Разрешение имён",
                            if self.dns_ok { "работает" } else { "сбой" },
                        );

                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if widgets::button(ui, "Обновить", Some(Icon::Reset), PillKind::Outlined, true)
                                .clicked()
                            {
                                self.engine.refresh_config();
                                self.engine.probe_health();
                            }
                            if widgets::button(ui, "Сбросить DNS", None, PillKind::Text, self.applied.is_some())
                                .clicked()
                            {
                                self.engine.reset(self.alias.clone());
                            }
                        });
                    });
                },
            );

            ui.add_space(gap);

            ui.allocate_ui_with_layout(
                Vec2::new(right_width, 0.0),
                Layout::top_down(Align::Min),
                |ui| {
                    widgets::card(ui, theme::SURFACE_CONTAINER, 24.0, |ui| {
                        ui.add(
                            egui::Label::new(theme::heading("О проекте", 17.0)).selectable(false),
                        );
                        ui.add_space(10.0);
                        ui.add(
                            egui::Label::new(
                                theme::subtle(
                                    "NeuroFix управляет DNS-резолверами активного подключения, измеряет задержку ответа и проверяет доступность AI-сервисов.",
                                    13.5,
                                ),
                            )
                            .wrap(),
                        );
                        ui.add_space(16.0);
                        info_row(ui, "Версия", &format!("v{}", VERSION));
                        info_row(ui, "Платформа", "Windows 10 / 11");
                        info_row(ui, "Стек", "Rust · egui");
                        info_row(ui, "Лицензия", "Apache 2.0");
                    });
                },
            );
        });

        ui.add_space(gap);
        widgets::card(ui, theme::SURFACE_LOW, 24.0, |ui| {
            ui.add(egui::Label::new(theme::heading("Возможности", 17.0)).selectable(false));
            ui.add_space(12.0);
            ui.columns(3, |columns| {
                feature(
                    &mut columns[0],
                    "Библиотека резолверов",
                    "Публичные DNS с адресами и быстрым поиском.",
                );
                feature(
                    &mut columns[1],
                    "Auto select",
                    "Замер задержки и выбор самого быстрого сервера.",
                );
                feature(
                    &mut columns[2],
                    "Проверка сервисов",
                    "Gemini, Claude и ChatGPT проверяются с этого компьютера.",
                );
            });
        });
    }

    fn dns_page(&mut self, ui: &mut Ui) {
        let mut action: Option<Action> = None;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                Self::header(
                    ui,
                    "DNS",
                    "Резолверы, задержка ответа и доступность сервисов",
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button(
                    ui,
                    "Добавить DNS",
                    Some(Icon::Plus),
                    PillKind::Outlined,
                    true,
                )
                .clicked()
                {
                    self.add_dialog = true;
                    self.dialog_error = None;
                }
            });
        });

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            let button_width = 188.0;
            let field_width = (ui.available_width() - button_width - 12.0).max(240.0);
            ui.allocate_ui_with_layout(
                Vec2::new(field_width, 52.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    widgets::search_field(ui, &mut self.search, "Поиск по названию или IP");
                },
            );
            ui.add_space(12.0);
            let enabled = !self.sweep_active;
            if widgets::button(
                ui,
                "Auto select",
                Some(Icon::Bolt),
                PillKind::Tonal,
                enabled,
            )
            .clicked()
            {
                action = Some(Action::Sweep);
            }
        });

        ui.add_space(10.0);

        let status = if self.sweep_active {
            Some(format!(
                "Замер задержки: {} из {}",
                self.views.len().saturating_sub(self.sweep_left),
                self.views.len()
            ))
        } else {
            self.applied
                .and_then(|id| self.views.iter().find(|view| view.resolver.id == id))
                .map(|view| format!("Активен: {}", view.resolver.name))
        };

        ui.horizontal(|ui| {
            if let Some(status) = status {
                ui.add(egui::Label::new(theme::subtle(&status, 13.0)).selectable(false));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                widgets::switch(ui, &mut self.auto_recovery);
                ui.add(
                    egui::Label::new(theme::subtle("Автовосстановление", 13.0)).selectable(false),
                );
                if self.applied.is_some()
                    && widgets::button(
                        ui,
                        "Сбросить DNS",
                        Some(Icon::Reset),
                        PillKind::Danger,
                        true,
                    )
                    .clicked()
                {
                    action = Some(Action::Reset);
                }
            });
        });

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let needle = self.search.trim().to_lowercase();
                let mut shown = 0;

                for view in &self.views {
                    if !needle.is_empty() {
                        let haystack = format!(
                            "{} {} {}",
                            view.resolver.name,
                            view.resolver.primary,
                            view.resolver
                                .secondary
                                .map(|ip| ip.to_string())
                                .unwrap_or_default()
                        )
                        .to_lowercase();
                        if !haystack.contains(&needle) {
                            continue;
                        }
                    }
                    shown += 1;
                    if let Some(next) = resolver_card(ui, view, self.applied) {
                        action = Some(next);
                    }
                    ui.add_space(14.0);
                }

                if shown == 0 {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.add(
                            egui::Label::new(theme::heading("Ничего не найдено", 18.0))
                                .selectable(false),
                        );
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(theme::subtle(
                                "Измените запрос или добавьте собственный резолвер",
                                13.0,
                            ))
                            .selectable(false),
                        );
                    });
                }
            });

        match action {
            Some(Action::Apply(resolver)) => {
                self.pending_apply = Some(resolver.id);
                self.engine.apply(resolver, self.alias.clone());
            }
            Some(Action::Check(resolver)) => {
                if let Some(view) = self.view_mut(resolver.id) {
                    view.services = [None, None, None];
                    view.checking_services = true;
                }
                self.engine.check_services(resolver);
            }
            Some(Action::Sweep) => self.start_sweep(true),
            Some(Action::Reset) => self.engine.reset(self.alias.clone()),
            None => {}
        }
    }

    fn vpn_page(ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(120.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(96.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, 32.0, theme::SECONDARY_CONTAINER);
            let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(46.0));
            icons::paint(
                ui.painter(),
                icon_rect,
                Icon::Shield,
                theme::ON_SECONDARY_CONTAINER,
                1.7,
            );
            ui.add_space(24.0);
            ui.add(egui::Label::new(theme::heading("VPN", 28.0)).selectable(false));
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(theme::heading("Скоро", 18.0).color(theme::ON_SURFACE_VARIANT))
                    .selectable(false),
            );
            ui.add_space(10.0);
            ui.add(
                egui::Label::new(theme::subtle("Раздел находится в разработке", 13.5))
                    .selectable(false),
            );
        });
    }

    fn about(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            widgets::card(ui, theme::SURFACE_LOW, 32.0, |ui| {
                ui.set_width(520.0);
                ui.vertical_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(64.0), Sense::hover());
                    ui.painter().rect_filled(rect, 20.0, theme::PRIMARY);
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "N",
                        FontId::new(30.0, theme::bold()),
                        theme::ON_PRIMARY,
                    );
                    ui.add_space(14.0);
                    ui.add(egui::Label::new(theme::heading("NeuroFix", 26.0)).selectable(false));
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(theme::subtle(&format!("Версия v{}", VERSION), 14.0))
                            .selectable(false),
                    );
                    ui.add_space(16.0);
                    ui.add(
                        egui::Label::new(
                            theme::subtle(
                                "Приложение для управления DNS-резолверами в Windows: выбор сервера, замер задержки, проверка доступности сервисов и автоматическое восстановление подключения.",
                                13.5,
                            ),
                        )
                        .wrap(),
                    );
                });
                ui.add_space(20.0);
                info_row(ui, "Версия", "v1.3.0");
                info_row(ui, "Сборка", "Rust · eframe · egui");
                info_row(ui, "Лицензия", "Apache License 2.0");
                info_row(ui, "Платформа", "Windows 10 / 11");
            });
            ui.add_space(18.0);
            ui.add(
                egui::Label::new(theme::subtle("© 2026 NeuroFix", 12.0)).selectable(false),
            );
        });
    }

    fn add_dialog(&mut self, ctx: &egui::Context) {
        let mut close = false;

        egui::Area::new(egui::Id::new("dim"))
            .fixed_pos(egui::Pos2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let screen = ctx.screen_rect();
                let (rect, response) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_black_alpha(96));
                if response.clicked() {
                    close = true;
                }
            });

        egui::Area::new(egui::Id::new("add-dialog"))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(theme::SURFACE_HIGH)
                    .rounding(28.0)
                    .inner_margin(egui::Margin::same(26.0))
                    .show(ui, |ui| {
                        ui.set_width(400.0);
                        ui.add(
                            egui::Label::new(theme::heading("Новый резолвер", 20.0))
                                .selectable(false),
                        );
                        ui.add_space(4.0);
                        ui.add(
                            egui::Label::new(theme::subtle(
                                "Укажите название и адреса DNS-серверов",
                                13.0,
                            ))
                            .selectable(false),
                        );
                        ui.add_space(16.0);

                        dialog_field(ui, "Название", &mut self.new_name, "Мой DNS");
                        ui.add_space(10.0);
                        dialog_field(ui, "Основной IP", &mut self.new_primary, "1.1.1.1");
                        ui.add_space(10.0);
                        dialog_field(
                            ui,
                            "Дополнительный IP",
                            &mut self.new_secondary,
                            "необязательно",
                        );

                        if let Some(error) = &self.dialog_error {
                            ui.add_space(10.0);
                            ui.add(
                                egui::Label::new(theme::body(error, 12.5).color(theme::ERROR))
                                    .selectable(false),
                            );
                        }

                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if widgets::button(
                                ui,
                                "Добавить",
                                Some(Icon::Plus),
                                PillKind::Filled,
                                true,
                            )
                            .clicked()
                            {
                                match (
                                    self.new_name.trim(),
                                    self.new_primary.trim().parse::<Ipv4Addr>(),
                                ) {
                                    ("", _) => {
                                        self.dialog_error = Some("Укажите название".to_owned())
                                    }
                                    (_, Err(_)) => {
                                        self.dialog_error =
                                            Some("Некорректный основной IP".to_owned())
                                    }
                                    (name, Ok(primary)) => {
                                        let secondary = self.new_secondary.trim();
                                        let secondary = if secondary.is_empty() {
                                            Ok(None)
                                        } else {
                                            secondary.parse::<Ipv4Addr>().map(Some)
                                        };
                                        match secondary {
                                            Err(_) => {
                                                self.dialog_error = Some(
                                                    "Некорректный дополнительный IP".to_owned(),
                                                )
                                            }
                                            Ok(secondary) => {
                                                let id = dns::new_id(
                                                    &self
                                                        .views
                                                        .iter()
                                                        .map(|view| view.resolver.clone())
                                                        .collect::<Vec<_>>(),
                                                );
                                                let resolver = Resolver {
                                                    id,
                                                    name: name.to_owned(),
                                                    primary,
                                                    secondary,
                                                    custom: true,
                                                };
                                                self.views
                                                    .push(ResolverView::new(resolver.clone()));
                                                let all: Vec<Resolver> = self
                                                    .views
                                                    .iter()
                                                    .map(|view| view.resolver.clone())
                                                    .collect();
                                                let _ = config::save_custom(&all);
                                                self.engine.measure(resolver.clone());
                                                self.engine.check_services(resolver);
                                                self.new_name.clear();
                                                self.new_primary.clear();
                                                self.new_secondary.clear();
                                                self.dialog_error = None;
                                                self.toast("Резолвер добавлен");
                                                close = true;
                                            }
                                        }
                                    }
                                }
                            }
                            if widgets::button(
                                ui,
                                "Отмена",
                                Some(Icon::Close),
                                PillKind::Text,
                                true,
                            )
                            .clicked()
                            {
                                close = true;
                            }
                        });
                    });
            });

        if close {
            self.add_dialog = false;
            self.dialog_error = None;
        }
    }

    fn snackbar(&mut self, ctx: &egui::Context) {
        let Some((message, since)) = self.toast.clone() else {
            return;
        };
        if since.elapsed() > Duration::from_secs(5) {
            self.toast = None;
            return;
        }

        let mut close = false;
        egui::Area::new(egui::Id::new("snackbar"))
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -28.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(theme::INVERSE_SURFACE)
                    .rounding(12.0)
                    .inner_margin(egui::Margin::symmetric(20.0, 14.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Label::new(
                                    theme::body(&message, 13.5).color(theme::ON_INVERSE_SURFACE),
                                )
                                .selectable(false),
                            );
                            ui.add_space(8.0);
                            if widgets::button(ui, "Скрыть", None, PillKind::Outlined, true)
                                .clicked()
                            {
                                close = true;
                            }
                        });
                    });
            });

        if close {
            self.toast = None;
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();

        egui::SidePanel::left("rail")
            .exact_width(272.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(theme::SURFACE_LOW)
                    .inner_margin(egui::Margin::symmetric(20.0, 24.0)),
            )
            .show(ctx, |ui| self.rail(ui));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::SURFACE)
                    .inner_margin(egui::Margin::symmetric(32.0, 26.0)),
            )
            .show(ctx, |ui| match self.page {
                Page::Home => self.home(ui),
                Page::Dns => self.dns_page(ui),
                Page::Vpn => Self::vpn_page(ui),
                Page::About => self.about(ui),
            });

        if self.add_dialog {
            self.add_dialog(ctx);
        }

        self.snackbar(ctx);

        if !self.sweep_active && self.views.iter().any(|view| view.checking_services) {
            ctx.request_repaint_after(Duration::from_millis(400));
        }
    }
}

fn info_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(theme::subtle(label, 13.0)).selectable(false));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(value)
                        .font(FontId::new(13.0, theme::bold()))
                        .color(theme::ON_SURFACE),
                )
                .selectable(false),
            );
        });
    });
}

fn feature(ui: &mut Ui, title: &str, text: &str) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            icons::paint(ui.painter(), rect, Icon::Check, theme::PRIMARY, 2.2);
            ui.add(egui::Label::new(theme::heading(title, 14.0)).selectable(false));
        });
        ui.add_space(4.0);
        ui.add(egui::Label::new(theme::subtle(text, 12.5)).wrap());
    });
}

fn dialog_field(ui: &mut Ui, label: &str, value: &mut String, hint: &str) {
    ui.add(egui::Label::new(theme::subtle(label, 12.0)).selectable(false));
    ui.add_space(4.0);
    egui::Frame::none()
        .fill(theme::SURFACE_LOWEST)
        .rounding(14.0)
        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
        .stroke(Stroke::new(1.0_f32, theme::OUTLINE_VARIANT))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .frame(false)
                    .desired_width(ui.available_width())
                    .hint_text(RichText::new(hint).color(theme::ON_SURFACE_VARIANT))
                    .font(FontId::new(14.0, FontFamily::Proportional)),
            );
        });
}

fn resolver_card(ui: &mut Ui, view: &ResolverView, applied: Option<usize>) -> Option<Action> {
    let is_applied = applied == Some(view.resolver.id);
    let fill = if is_applied {
        theme::PRIMARY_CONTAINER
    } else {
        theme::SURFACE_LOW
    };
    let stroke = if is_applied {
        Stroke::new(1.4_f32, theme::PRIMARY)
    } else {
        Stroke::new(1.0_f32, theme::OUTLINE_VARIANT)
    };

    let mut action = None;

    egui::Frame::none()
        .fill(fill)
        .rounding(theme::RADIUS_L)
        .stroke(stroke)
        .inner_margin(egui::Margin::symmetric(22.0, 18.0))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let right_width = 172.0;
                let left_width = (ui.available_width() - right_width - 16.0).max(240.0);

                ui.allocate_ui_with_layout(
                    Vec2::new(left_width, 0.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Label::new(theme::heading(&view.resolver.name, 17.0))
                                    .selectable(false),
                            );
                            if is_applied {
                                widgets::chip(ui, "Активен", theme::PRIMARY, theme::ON_PRIMARY);
                            }
                        });

                        let secondary = view
                            .resolver
                            .secondary
                            .map(|ip| format!("   ·   {}", ip))
                            .unwrap_or_default();
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{}{}", view.resolver.primary, secondary))
                                    .font(FontId::new(13.0, FontFamily::Monospace))
                                    .color(theme::ON_SURFACE_VARIANT),
                            )
                            .selectable(false),
                        );

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            for (index, service) in SERVICES.iter().enumerate() {
                                let (label, bg, fg) = match view.services[index] {
                                    Some(true) => (
                                        format!("{}: YES", service.name),
                                        theme::OK_CONTAINER,
                                        theme::ON_OK_CONTAINER,
                                    ),
                                    Some(false) => (
                                        format!("{}: NO", service.name),
                                        theme::ERROR_CONTAINER,
                                        theme::ON_ERROR_CONTAINER,
                                    ),
                                    None => (
                                        format!("{}: ...", service.name),
                                        theme::SURFACE_HIGH,
                                        theme::ON_SURFACE_VARIANT,
                                    ),
                                };
                                widgets::chip(ui, &label, bg, fg);
                            }
                        });
                    },
                );

                ui.allocate_ui_with_layout(
                    Vec2::new(right_width, 0.0),
                    Layout::top_down(Align::Max),
                    |ui| {
                        let latency = if view.checking_latency && view.latency.is_none() {
                            "замер...".to_owned()
                        } else {
                            view.latency
                                .map(|ms| format!("{} мс", ms))
                                .unwrap_or_else(|| "нет ответа".to_owned())
                        };
                        ui.add(
                            egui::Label::new(
                                RichText::new(latency)
                                    .font(FontId::new(13.5, theme::bold()))
                                    .color(if view.latency.is_some() {
                                        theme::ON_SURFACE
                                    } else {
                                        theme::ON_SURFACE_VARIANT
                                    }),
                            )
                            .selectable(false),
                        );
                        ui.add_space(8.0);

                        let busy = view.checking_services;
                        let label = if is_applied {
                            "Применён"
                        } else {
                            "Применить"
                        };
                        if widgets::button(
                            ui,
                            label,
                            None,
                            if is_applied {
                                PillKind::Outlined
                            } else {
                                PillKind::Filled
                            },
                            !is_applied && !busy,
                        )
                        .clicked()
                        {
                            action = Some(Action::Apply(view.resolver.clone()));
                        }

                        ui.add_space(4.0);
                        if widgets::button(
                            ui,
                            if busy {
                                "Проверка..."
                            } else {
                                "Проверить"
                            },
                            None,
                            PillKind::Text,
                            !busy,
                        )
                        .clicked()
                        {
                            action = Some(Action::Check(view.resolver.clone()));
                        }
                    },
                );
            });
        });

    action
}
