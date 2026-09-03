// core/hypervisor/src/hud/navigation/palette.rs
//! Global Command Palette (`Ctrl+K` / `Ctrl+P`) with fuzzy command search.

use crate::hud::navigation::NavSection;
use crate::hud::theme::HudTheme;
use eframe::egui::{self, Color32, CornerRadius, Key, Stroke};
use std::path::PathBuf;

/// Command Action for the Global Command Palette
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    Navigate(NavSection),
    ToggleRecording,
    ToggleCompactOverlay,
    MinimizeToTray,
    ToggleInGameOverlay,
    ToggleDevMode,
    RunDiagnostics,
    RescanModels,
    MineSiDistillation,
    RunSiMacro(String, PathBuf),
    TileWindowsGrid,
    SetTheme(HudTheme),
}

#[derive(Default)]
pub struct CommandPalette {
    pub is_open: bool,
    pub query: String,
    pub selected_idx: usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            selected_idx: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open {
            self.query.clear();
            self.selected_idx = 0;
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        theme: HudTheme,
    ) -> Option<CommandAction> {
        if !self.is_open {
            return None;
        }

        let all_commands = vec![
            ("👥 Specialists Deck", "Open Domain Specialists and Hive routing", CommandAction::Navigate(NavSection::Specialists)),
            ("🌌 3D Omni Galaxy Graph", "Explore 3D spatial knowledge graph", CommandAction::Navigate(NavSection::GalaxyMap3D)),
            ("🧬 Learning & Self-Play", "View neurochemistry & Alice vs Bob dreams", CommandAction::Navigate(NavSection::LearningAndSelfPlay)),
            ("⚡ Solid-State SI Forge", "Compile native .si models & AST diffs", CommandAction::Navigate(NavSection::SiForge)),
            ("👁️ Screen & Motor Engine", "Discord-style window picker & vision", CommandAction::Navigate(NavSection::ScreenAutomation)),
            ("🌐 Federation P2P Swarm", "Multi-hive quorum & work offloading", CommandAction::Navigate(NavSection::SwarmMesh)),
            ("🤖 SI Agents & Workflows", "Create and run persistent automation bots", CommandAction::Navigate(NavSection::Agents)),
            ("⚙️ Preferences & Model Hub", "Configure themes and scan local GGUF models", CommandAction::Navigate(NavSection::Settings)),
            ("🛠️ Developer Workbench", "Source tree, code editor & live diffs", CommandAction::Navigate(NavSection::DevStudio)),
            ("⚡ Shared Memory Bus Monitor", "Inspect zero-copy SWMR 64MB ring buffer", CommandAction::Navigate(NavSection::InterconnectMonitor)),
            ("💬 Internal IPC Chat", "Send task intents directly into bus", CommandAction::Navigate(NavSection::Console)),
            ("🔴 Toggle Macro Recording (F9)", "Start or stop live action demonstration", CommandAction::ToggleRecording),
            ("🪟 Toggle Compact Mini-HUD (F10)", "Switch between full studio and floating recorder", CommandAction::ToggleCompactOverlay),
            ("🎮 In-Game Transparent Overlay (F12)", "Launch transparent bot overlay", CommandAction::ToggleInGameOverlay),
            ("🔄 Rescan All GGUF Model Hubs", "Auto-discover LM Studio, Ollama, HuggingFace", CommandAction::RescanModels),
            ("💎 Mine SI Distillation Corpus", "Extract high-density synthetic reasoning traces", CommandAction::MineSiDistillation),
            ("📐 Tile Windows in 2-Col Grid", "Zero-overlap arrangement for dynamic tools", CommandAction::TileWindowsGrid),
            ("🎨 Theme: Cobalt Dark", "Switch to Cobalt Dark theme", CommandAction::SetTheme(HudTheme::CobaltDark)),
            ("🎨 Theme: Obsidian Slate", "Switch to Obsidian Slate theme", CommandAction::SetTheme(HudTheme::ObsidianSlate)),
            ("🎨 Theme: Emerald Matrix", "Switch to Emerald Matrix theme", CommandAction::SetTheme(HudTheme::EmeraldMatrix)),
            ("🎨 Theme: Amber Sovereign", "Switch to Amber Sovereign theme", CommandAction::SetTheme(HudTheme::AmberSovereign)),
        ];

        let query_lower = self.query.to_lowercase();
        let filtered: Vec<_> = all_commands
            .into_iter()
            .filter(|(name, desc, _)| {
                query_lower.is_empty()
                    || name.to_lowercase().contains(&query_lower)
                    || desc.to_lowercase().contains(&query_lower)
            })
            .collect();

        let mut executed_action = None;
        let screen_rect = ctx.content_rect();
        let modal_width = 560.0f32.min(screen_rect.width() - 40.0);
        let modal_height = 380.0f32.min(screen_rect.height() - 80.0);

        let area_res = egui::Area::new(egui::Id::new("command_palette_modal"))
            .fixed_pos(egui::pos2(
                (screen_rect.width() - modal_width) * 0.5,
                screen_rect.height() * 0.15,
            ))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::window(&ctx.global_style())
                    .fill(theme.panel_bg())
                    .stroke(Stroke::new(1.5, theme.accent()))
                    .corner_radius(CornerRadius::same(10))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 4,
                        color: Color32::from_black_alpha(180),
                    })
                    .show(ui, |ui| {
                        ui.set_width(modal_width);
                        ui.set_height(modal_height);

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("🔍")
                                    .size(18.0)
                                    .color(theme.accent()),
                            );
                            let text_edit = egui::TextEdit::singleline(&mut self.query)
                                .hint_text("Type a command or navigate... (↑/↓ to navigate, Enter to run, Esc to close)")
                                .desired_width(modal_width - 60.0)
                                .font(egui::TextStyle::Heading);
                            let response = ui.add(text_edit);
                            response.request_focus();

                            if ui.input(|i| i.key_pressed(Key::Escape)) {
                                self.is_open = false;
                            }
                        });

                        ui.separator();

                        if filtered.is_empty() {
                            ui.add_space(20.0);
                            ui.label(
                                egui::RichText::new("No matching commands found.")
                                    .color(Color32::GRAY)
                                    .italics(),
                            );
                        } else {
                            if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                                self.selected_idx = (self.selected_idx + 1) % filtered.len();
                            }
                            if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                                self.selected_idx = if self.selected_idx == 0 {
                                    filtered.len() - 1
                                } else {
                                    self.selected_idx - 1
                                };
                            }
                            if ui.input(|i| i.key_pressed(Key::Enter)) {
                                if let Some((_, _, action)) = filtered.get(self.selected_idx) {
                                    executed_action = Some(action.clone());
                                    self.is_open = false;
                                }
                            }

                            egui::ScrollArea::vertical().max_height(modal_height - 60.0).show(ui, |ui| {
                                for (i, (name, desc, action)) in filtered.iter().enumerate() {
                                    let is_selected = i == self.selected_idx;
                                    let bg_color = if is_selected {
                                        theme.card_bg()
                                    } else {
                                        Color32::TRANSPARENT
                                    };

                                    let item_resp = egui::Frame::group(ui.style())
                                        .fill(bg_color)
                                        .stroke(if is_selected {
                                            Stroke::new(1.0, theme.accent())
                                        } else {
                                            Stroke::NONE
                                        })
                                        .corner_radius(CornerRadius::same(6))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(*name)
                                                            .strong()
                                                            .size(13.0)
                                                            .color(if is_selected {
                                                                theme.accent()
                                                            } else {
                                                                Color32::WHITE
                                                            }),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(*desc)
                                                            .size(11.0)
                                                            .color(Color32::GRAY),
                                                    );
                                                });
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if is_selected {
                                                            ui.label(
                                                                egui::RichText::new("↵ Enter")
                                                                    .size(10.0)
                                                                    .color(theme.accent()),
                                                            );
                                                        }
                                                    },
                                                );
                                            });
                                        });

                                    if item_resp.response.clicked() {
                                        executed_action = Some(action.clone());
                                        self.is_open = false;
                                    }
                                }
                            });
                        }
                    });
            });

        if area_res.response.clicked_elsewhere() && !self.query.is_empty() {
            // keep open if actively searching, close on outside click
        }

        executed_action
    }
}
