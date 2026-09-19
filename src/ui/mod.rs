//! UI-panelen van de applicatie.

pub mod about;
pub mod details_panel;
pub mod drives_panel;
pub mod log_panel;
pub mod settings_panel;
pub mod top_bar;

use crate::app::Theme;

/// Past het kleurthema toe (stap 10b-8). Dark is de bestaande look; Soft
/// houdt de donkere basis maar licht de panelen op; Light is het lichte
/// egui-thema met aangepaste paneelvulling; HighContrast is zwart met pure
/// witte tekst.
///
/// Belangrijk: `all_styles_mut` raakt BEIDE stijlen (dark + light) — dus
/// elke toepassing begint met de egui-defaults voor alles wat een thema
/// kan aanpassen. Zonder die reset sluimeren de witte HighContrast-tekst-
/// kleuren door in de light-stijl (en via eframe-persistence ook over
/// herstarts heen).
pub fn apply_theme(ctx: &egui::Context, theme: Theme) {
    ctx.set_theme(match theme {
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark | Theme::Soft | Theme::HighContrast => egui::ThemePreference::Dark,
    });
    ctx.all_styles_mut(|style| {
        // 1) Terug naar de egui-defaults voor de onderdelen die thema's
        //    aanpassen (per stijl de juiste default: dark of light).
        let defaults = if style.visuals.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        style.visuals.hyperlink_color = defaults.hyperlink_color;
        for (widget, default) in [
            (
                &mut style.visuals.widgets.noninteractive,
                &defaults.widgets.noninteractive,
            ),
            (
                &mut style.visuals.widgets.inactive,
                &defaults.widgets.inactive,
            ),
            (
                &mut style.visuals.widgets.hovered,
                &defaults.widgets.hovered,
            ),
            (&mut style.visuals.widgets.active, &defaults.widgets.active),
            (&mut style.visuals.widgets.open, &defaults.widgets.open),
        ] {
            widget.bg_fill = default.bg_fill;
            widget.weak_bg_fill = default.weak_bg_fill;
            widget.bg_stroke = default.bg_stroke;
            widget.fg_stroke = default.fg_stroke;
        }

        // 2) Algemene spacing.
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);

        // 3) Thema-specifieke delta.
        match theme {
            Theme::Dark => {
                style.visuals.panel_fill = egui::Color32::from_rgb(24, 26, 31);
            }
            Theme::Soft => {
                style.visuals.panel_fill = egui::Color32::from_rgb(43, 46, 54);
            }
            Theme::Light => {
                style.visuals.panel_fill = egui::Color32::from_rgb(238, 240, 244);
            }
            Theme::HighContrast => {
                style.visuals.panel_fill = egui::Color32::BLACK;
                // Maximale leesbaarheid: élke widget-tekst in puur wit
                // (text_color() leest fg_stroke.color).
                for widget in [
                    &mut style.visuals.widgets.noninteractive,
                    &mut style.visuals.widgets.inactive,
                    &mut style.visuals.widgets.hovered,
                    &mut style.visuals.widgets.active,
                    &mut style.visuals.widgets.open,
                ] {
                    widget.fg_stroke.color = egui::Color32::WHITE;
                }
                style.visuals.hyperlink_color = egui::Color32::from_rgb(150, 195, 255);
            }
        }
    });
}

/// Primaire actie-knop in de accentkleur met contrast-tekst — voor de
/// hoofdacties per eiland (inspecteren, branden, samenstellen, kopiëren).
pub fn primary_button<'a>(p: &'a colors::Palette, text: &'a str) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(text).strong().color(p.button_text)).fill(p.accent)
}

/// Thema-bewuste semantische kleuren (stap 10b-8): de donkere set is de
/// originele, de lichte set is donkerder getint voor leesbaarheid op de
/// lichte achtergrond. De keuze volgt de actieve egui-themavoorkeur (Dark
/// voor Dark/Soft/HighContrast, Light voor Light).
pub mod colors {
    use egui::Color32;

    pub struct Palette {
        /// Succes / gezonde toestand.
        pub ok: Color32,
        /// Waarschuwing / bezig.
        pub warn: Color32,
        /// Fout.
        pub bad: Color32,
        /// Secundaire tekst.
        pub dim: Color32,
        /// Info-niveau in de log (tussen dim en normale tekst).
        pub info: Color32,
        /// Accentkleur voor primaire knoppen.
        pub accent: Color32,
        /// Tekstkleur op accentknoppen (contrast met `accent`).
        pub button_text: Color32,
        /// Fel oranje voor nadruk (bijv. de brandsnelheid tijdens het branden).
        pub orange: Color32,
    }

    pub fn palette(ctx: &egui::Context) -> Palette {
        if ctx.theme() == egui::Theme::Light {
            Palette {
                ok: Color32::from_rgb(24, 128, 56),
                warn: Color32::from_rgb(158, 104, 0),
                bad: Color32::from_rgb(190, 32, 44),
                dim: Color32::from_rgb(92, 99, 110),
                info: Color32::from_rgb(96, 106, 122),
                accent: Color32::from_rgb(38, 96, 190),
                orange: Color32::from_rgb(190, 88, 0),
                button_text: Color32::WHITE,
            }
        } else {
            Palette {
                ok: Color32::from_rgb(108, 200, 128),
                warn: Color32::from_rgb(232, 182, 92),
                bad: Color32::from_rgb(238, 112, 112),
                dim: Color32::from_rgb(120, 128, 140),
                info: Color32::from_rgb(140, 158, 178),
                accent: Color32::from_rgb(120, 170, 250),
                orange: Color32::from_rgb(255, 145, 30),
                button_text: Color32::from_rgb(18, 22, 30),
            }
        }
    }

    /// Logkleur per niveau.
    impl Palette {
        pub fn level(&self, level: crate::logger::Level) -> Color32 {
            use crate::logger::Level;
            match level {
                Level::Info => self.info,
                Level::Success => self.ok,
                Level::Warning => self.warn,
                Level::Error => self.bad,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn palette_follows_resolved_theme() {
            let ctx = egui::Context::default();

            ctx.set_theme(egui::ThemePreference::Light);
            let light = palette(&ctx);
            assert_eq!(light.accent, Color32::from_rgb(38, 96, 190));
            assert_eq!(light.ok, Color32::from_rgb(24, 128, 56));
            assert_eq!(light.button_text, Color32::WHITE);

            ctx.set_theme(egui::ThemePreference::Dark);
            let dark = palette(&ctx);
            assert_eq!(dark.accent, Color32::from_rgb(120, 170, 250));
            assert_eq!(dark.ok, Color32::from_rgb(108, 200, 128));
        }
    }
}

/// Kleine schuif-schakelaar (aan/uit) in de paneelstijl — een fraaier
/// alternatief voor een checkbox bij positief geformuleerde opties.
pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired = egui::vec2(28.0, 15.0);
    let (rect, mut response) = ui.allocate_exact_size(desired, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    // Vloeiende overgang van de knop tussen uit- en aan-stand.
    let t = ui.ctx().animate_bool_with_time(response.id, *on, 0.12);
    let pal = colors::palette(ui.ctx());
    let track = if *on { pal.ok } else { pal.dim };
    let track = track.gamma_multiply(0.35 + 0.65 * t);
    let knob_x = egui::lerp(rect.left() + 7.5..=rect.right() - 7.5, t);
    let center = egui::pos2(knob_x, rect.center().y);

    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(7), track);
    ui.painter()
        .circle_filled(center, 5.0, egui::Color32::WHITE);

    // Builder-stijl: on_hover_cursor neemt self — dus als laatste stap.
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use crate::app::Theme;

    /// Regressie: HighContrast zet witte widget-tekst in BEIDE stijlen
    /// (`all_styles_mut`); na terugschakelen naar Light mag die niet
    /// doorsluimeren — anders witte letters op lichte panelen.
    #[test]
    fn high_contrast_does_not_leak_into_light() {
        let ctx = egui::Context::default();
        apply_theme(&ctx, Theme::Light);
        apply_theme(&ctx, Theme::HighContrast);
        apply_theme(&ctx, Theme::Light);

        let style = ctx.style_of(egui::Theme::Light);
        assert_eq!(
            style.visuals.panel_fill,
            egui::Color32::from_rgb(238, 240, 244)
        );
        assert_ne!(
            style.visuals.widgets.noninteractive.fg_stroke.color,
            egui::Color32::WHITE
        );
        assert_ne!(
            style.visuals.widgets.inactive.fg_stroke.color,
            egui::Color32::WHITE
        );
    }
}
