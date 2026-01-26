use bevy_egui::egui;
use aracari::prelude::*;

const MENU_WIDTH: f32 = 140.0;

fn remove_button_borders(ui: &mut egui::Ui) {
    let visuals = ui.visuals_mut();
    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EasingType {
    Sine,
    Quad,
    Cubic,
    Quart,
    Quint,
    Expo,
    Circ,
    Back,
    Elastic,
    Bounce,
}

impl EasingType {
    fn label(&self) -> &'static str {
        match self {
            Self::Sine => "Sine",
            Self::Quad => "Quad",
            Self::Cubic => "Cubic",
            Self::Quart => "Quart",
            Self::Quint => "Quint",
            Self::Expo => "Expo",
            Self::Circ => "Circ",
            Self::Back => "Back",
            Self::Elastic => "Elastic",
            Self::Bounce => "Bounce",
        }
    }

    fn all() -> &'static [EasingType] {
        &[
            Self::Sine,
            Self::Quad,
            Self::Cubic,
            Self::Quart,
            Self::Quint,
            Self::Expo,
            Self::Circ,
            Self::Back,
            Self::Elastic,
            Self::Bounce,
        ]
    }

    fn to_curve(&self, direction: EasingDirection, mode: EasingMode) -> SplineCurve {
        match (direction, mode, self) {
            // increase - ease in
            (EasingDirection::Increase, EasingMode::In, Self::Sine) => SplineCurve::SineIn,
            (EasingDirection::Increase, EasingMode::In, Self::Quad) => SplineCurve::QuadIn,
            (EasingDirection::Increase, EasingMode::In, Self::Cubic) => SplineCurve::CubicIn,
            (EasingDirection::Increase, EasingMode::In, Self::Quart) => SplineCurve::QuartIn,
            (EasingDirection::Increase, EasingMode::In, Self::Quint) => SplineCurve::QuintIn,
            (EasingDirection::Increase, EasingMode::In, Self::Expo) => SplineCurve::ExpoIn,
            (EasingDirection::Increase, EasingMode::In, Self::Circ) => SplineCurve::CircIn,
            (EasingDirection::Increase, EasingMode::In, Self::Back) => SplineCurve::BackIn,
            (EasingDirection::Increase, EasingMode::In, Self::Elastic) => SplineCurve::ElasticIn,
            (EasingDirection::Increase, EasingMode::In, Self::Bounce) => SplineCurve::BounceIn,

            // increase - ease out
            (EasingDirection::Increase, EasingMode::Out, Self::Sine) => SplineCurve::SineOut,
            (EasingDirection::Increase, EasingMode::Out, Self::Quad) => SplineCurve::QuadOut,
            (EasingDirection::Increase, EasingMode::Out, Self::Cubic) => SplineCurve::CubicOut,
            (EasingDirection::Increase, EasingMode::Out, Self::Quart) => SplineCurve::QuartOut,
            (EasingDirection::Increase, EasingMode::Out, Self::Quint) => SplineCurve::QuintOut,
            (EasingDirection::Increase, EasingMode::Out, Self::Expo) => SplineCurve::ExpoOut,
            (EasingDirection::Increase, EasingMode::Out, Self::Circ) => SplineCurve::CircOut,
            (EasingDirection::Increase, EasingMode::Out, Self::Back) => SplineCurve::BackOut,
            (EasingDirection::Increase, EasingMode::Out, Self::Elastic) => SplineCurve::ElasticOut,
            (EasingDirection::Increase, EasingMode::Out, Self::Bounce) => SplineCurve::BounceOut,

            // increase - ease in-out
            (EasingDirection::Increase, EasingMode::InOut, Self::Sine) => SplineCurve::SineInOut,
            (EasingDirection::Increase, EasingMode::InOut, Self::Quad) => SplineCurve::QuadInOut,
            (EasingDirection::Increase, EasingMode::InOut, Self::Cubic) => SplineCurve::CubicInOut,
            (EasingDirection::Increase, EasingMode::InOut, Self::Quart) => SplineCurve::QuartInOut,
            (EasingDirection::Increase, EasingMode::InOut, Self::Quint) => SplineCurve::QuintInOut,
            (EasingDirection::Increase, EasingMode::InOut, Self::Expo) => SplineCurve::ExpoInOut,
            (EasingDirection::Increase, EasingMode::InOut, Self::Circ) => SplineCurve::CircInOut,
            (EasingDirection::Increase, EasingMode::InOut, Self::Back) => SplineCurve::BackInOut,
            (EasingDirection::Increase, EasingMode::InOut, Self::Elastic) => {
                SplineCurve::ElasticInOut
            }
            (EasingDirection::Increase, EasingMode::InOut, Self::Bounce) => {
                SplineCurve::BounceInOut
            }

            // decrease - ease in
            (EasingDirection::Decrease, EasingMode::In, Self::Sine) => SplineCurve::SineInReverse,
            (EasingDirection::Decrease, EasingMode::In, Self::Quad) => SplineCurve::QuadInReverse,
            (EasingDirection::Decrease, EasingMode::In, Self::Cubic) => SplineCurve::CubicInReverse,
            (EasingDirection::Decrease, EasingMode::In, Self::Quart) => SplineCurve::QuartInReverse,
            (EasingDirection::Decrease, EasingMode::In, Self::Quint) => SplineCurve::QuintInReverse,
            (EasingDirection::Decrease, EasingMode::In, Self::Expo) => SplineCurve::ExpoInReverse,
            (EasingDirection::Decrease, EasingMode::In, Self::Circ) => SplineCurve::CircInReverse,
            (EasingDirection::Decrease, EasingMode::In, Self::Back) => SplineCurve::BackInReverse,
            (EasingDirection::Decrease, EasingMode::In, Self::Elastic) => SplineCurve::ElasticInReverse,
            (EasingDirection::Decrease, EasingMode::In, Self::Bounce) => SplineCurve::BounceInReverse,

            // decrease - ease out
            (EasingDirection::Decrease, EasingMode::Out, Self::Sine) => SplineCurve::SineOutReverse,
            (EasingDirection::Decrease, EasingMode::Out, Self::Quad) => SplineCurve::QuadOutReverse,
            (EasingDirection::Decrease, EasingMode::Out, Self::Cubic) => SplineCurve::CubicOutReverse,
            (EasingDirection::Decrease, EasingMode::Out, Self::Quart) => SplineCurve::QuartOutReverse,
            (EasingDirection::Decrease, EasingMode::Out, Self::Quint) => SplineCurve::QuintOutReverse,
            (EasingDirection::Decrease, EasingMode::Out, Self::Expo) => SplineCurve::ExpoOutReverse,
            (EasingDirection::Decrease, EasingMode::Out, Self::Circ) => SplineCurve::CircOutReverse,
            (EasingDirection::Decrease, EasingMode::Out, Self::Back) => SplineCurve::BackOutReverse,
            (EasingDirection::Decrease, EasingMode::Out, Self::Elastic) => {
                SplineCurve::ElasticOutReverse
            }
            (EasingDirection::Decrease, EasingMode::Out, Self::Bounce) => SplineCurve::BounceOutReverse,

            // decrease - ease in-out
            (EasingDirection::Decrease, EasingMode::InOut, Self::Sine) => SplineCurve::SineInOutReverse,
            (EasingDirection::Decrease, EasingMode::InOut, Self::Quad) => SplineCurve::QuadInOutReverse,
            (EasingDirection::Decrease, EasingMode::InOut, Self::Cubic) => {
                SplineCurve::CubicInOutReverse
            }
            (EasingDirection::Decrease, EasingMode::InOut, Self::Quart) => {
                SplineCurve::QuartInOutReverse
            }
            (EasingDirection::Decrease, EasingMode::InOut, Self::Quint) => {
                SplineCurve::QuintInOutReverse
            }
            (EasingDirection::Decrease, EasingMode::InOut, Self::Expo) => SplineCurve::ExpoInOutReverse,
            (EasingDirection::Decrease, EasingMode::InOut, Self::Circ) => SplineCurve::CircInOutReverse,
            (EasingDirection::Decrease, EasingMode::InOut, Self::Back) => SplineCurve::BackInOutReverse,
            (EasingDirection::Decrease, EasingMode::InOut, Self::Elastic) => {
                SplineCurve::ElasticInOutReverse
            }
            (EasingDirection::Decrease, EasingMode::InOut, Self::Bounce) => {
                SplineCurve::BounceInOutReverse
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EasingDirection {
    Increase,
    Decrease,
}

impl EasingDirection {
    fn label(&self) -> &'static str {
        match self {
            Self::Increase => "Increase",
            Self::Decrease => "Decrease",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EasingMode {
    In,
    Out,
    InOut,
}

impl EasingMode {
    fn label(&self) -> &'static str {
        match self {
            Self::In => "Ease in",
            Self::Out => "Ease out",
            Self::InOut => "Ease in-out",
        }
    }

    fn all() -> &'static [EasingMode] {
        &[Self::In, Self::Out, Self::InOut]
    }
}

pub fn spline_curve_config_label(config: &Option<SplineCurveConfig>) -> &'static str {
    match config {
        None => "Constant",
        Some(c) => spline_curve_label(&c.curve),
    }
}

pub fn spline_curve_label(curve: &SplineCurve) -> &'static str {
    match curve {
        SplineCurve::Constant => "Constant",

        // increase - linear
        SplineCurve::LinearIn => "Linear (Increase)",
        SplineCurve::LinearReverse => "Linear (Decrease)",

        // increase - sine
        SplineCurve::SineIn => "Sine In (Increase)",
        SplineCurve::SineOut => "Sine Out (Increase)",
        SplineCurve::SineInOut => "Sine In-Out (Increase)",

        // increase - quad
        SplineCurve::QuadIn => "Quad In (Increase)",
        SplineCurve::QuadOut => "Quad Out (Increase)",
        SplineCurve::QuadInOut => "Quad In-Out (Increase)",

        // increase - cubic
        SplineCurve::CubicIn => "Cubic In (Increase)",
        SplineCurve::CubicOut => "Cubic Out (Increase)",
        SplineCurve::CubicInOut => "Cubic In-Out (Increase)",

        // increase - quart
        SplineCurve::QuartIn => "Quart In (Increase)",
        SplineCurve::QuartOut => "Quart Out (Increase)",
        SplineCurve::QuartInOut => "Quart In-Out (Increase)",

        // increase - quint
        SplineCurve::QuintIn => "Quint In (Increase)",
        SplineCurve::QuintOut => "Quint Out (Increase)",
        SplineCurve::QuintInOut => "Quint In-Out (Increase)",

        // increase - expo
        SplineCurve::ExpoIn => "Expo In (Increase)",
        SplineCurve::ExpoOut => "Expo Out (Increase)",
        SplineCurve::ExpoInOut => "Expo In-Out (Increase)",

        // increase - circ
        SplineCurve::CircIn => "Circ In (Increase)",
        SplineCurve::CircOut => "Circ Out (Increase)",
        SplineCurve::CircInOut => "Circ In-Out (Increase)",

        // increase - back
        SplineCurve::BackIn => "Back In (Increase)",
        SplineCurve::BackOut => "Back Out (Increase)",
        SplineCurve::BackInOut => "Back In-Out (Increase)",

        // increase - elastic
        SplineCurve::ElasticIn => "Elastic In (Increase)",
        SplineCurve::ElasticOut => "Elastic Out (Increase)",
        SplineCurve::ElasticInOut => "Elastic In-Out (Increase)",

        // increase - bounce
        SplineCurve::BounceIn => "Bounce In (Increase)",
        SplineCurve::BounceOut => "Bounce Out (Increase)",
        SplineCurve::BounceInOut => "Bounce In-Out (Increase)",

        // decrease - sine
        SplineCurve::SineInReverse => "Sine In (Decrease)",
        SplineCurve::SineOutReverse => "Sine Out (Decrease)",
        SplineCurve::SineInOutReverse => "Sine In-Out (Decrease)",

        // decrease - quad
        SplineCurve::QuadInReverse => "Quad In (Decrease)",
        SplineCurve::QuadOutReverse => "Quad Out (Decrease)",
        SplineCurve::QuadInOutReverse => "Quad In-Out (Decrease)",

        // decrease - cubic
        SplineCurve::CubicInReverse => "Cubic In (Decrease)",
        SplineCurve::CubicOutReverse => "Cubic Out (Decrease)",
        SplineCurve::CubicInOutReverse => "Cubic In-Out (Decrease)",

        // decrease - quart
        SplineCurve::QuartInReverse => "Quart In (Decrease)",
        SplineCurve::QuartOutReverse => "Quart Out (Decrease)",
        SplineCurve::QuartInOutReverse => "Quart In-Out (Decrease)",

        // decrease - quint
        SplineCurve::QuintInReverse => "Quint In (Decrease)",
        SplineCurve::QuintOutReverse => "Quint Out (Decrease)",
        SplineCurve::QuintInOutReverse => "Quint In-Out (Decrease)",

        // decrease - expo
        SplineCurve::ExpoInReverse => "Expo In (Decrease)",
        SplineCurve::ExpoOutReverse => "Expo Out (Decrease)",
        SplineCurve::ExpoInOutReverse => "Expo In-Out (Decrease)",

        // decrease - circ
        SplineCurve::CircInReverse => "Circ In (Decrease)",
        SplineCurve::CircOutReverse => "Circ Out (Decrease)",
        SplineCurve::CircInOutReverse => "Circ In-Out (Decrease)",

        // decrease - back
        SplineCurve::BackInReverse => "Back In (Decrease)",
        SplineCurve::BackOutReverse => "Back Out (Decrease)",
        SplineCurve::BackInOutReverse => "Back In-Out (Decrease)",

        // decrease - elastic
        SplineCurve::ElasticInReverse => "Elastic In (Decrease)",
        SplineCurve::ElasticOutReverse => "Elastic Out (Decrease)",
        SplineCurve::ElasticInOutReverse => "Elastic In-Out (Decrease)",

        // decrease - bounce
        SplineCurve::BounceInReverse => "Bounce In (Decrease)",
        SplineCurve::BounceOutReverse => "Bounce Out (Decrease)",
        SplineCurve::BounceInOutReverse => "Bounce In-Out (Decrease)",

        // custom (not shown in editor picker)
        SplineCurve::Custom(_) => "Custom",
    }
}

/// Shows a ComboBox with nested submenus for selecting spline curve presets.
/// Returns true if the value changed.
pub fn spline_curve_config_picker(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut Option<SplineCurveConfig>,
    width: f32,
) -> bool {
    let mut changed = false;
    let current_text = spline_curve_config_label(value);

    // preserve min/max when changing curves
    let (current_min, current_max) = value
        .as_ref()
        .map(|c| (c.min_value, c.max_value))
        .unwrap_or((0.0, 1.0));

    egui::ComboBox::from_id_salt(id)
        .selected_text(current_text)
        .width(width)
        .show_ui(ui, |ui| {
            ui.set_min_width(MENU_WIDTH);
            remove_button_borders(ui);

            // constant option
            if ui
                .selectable_label(
                    matches!(value, None) || matches!(value, Some(c) if c.curve == SplineCurve::Constant),
                    "Constant",
                )
                .clicked()
            {
                *value = None;
                changed = true;
            }

            // increase direction submenu
            ui.menu_button(EasingDirection::Increase.label(), |ui| {
                ui.set_min_width(MENU_WIDTH);
                remove_button_borders(ui);

                // linear option
                if ui
                    .selectable_label(
                        matches!(value, Some(c) if c.curve == SplineCurve::LinearIn),
                        "Linear",
                    )
                    .clicked()
                {
                    *value = Some(SplineCurveConfig {
                        curve: SplineCurve::LinearIn,
                        min_value: current_min,
                        max_value: current_max,
                    });
                    changed = true;
                    ui.close();
                }

                for mode in EasingMode::all() {
                    ui.menu_button(mode.label(), |ui| {
                        ui.set_min_width(MENU_WIDTH);
                        remove_button_borders(ui);
                        for easing_type in EasingType::all() {
                            let curve = easing_type.to_curve(EasingDirection::Increase, *mode);
                            if ui
                                .selectable_label(
                                    matches!(value, Some(c) if c.curve == curve),
                                    easing_type.label(),
                                )
                                .clicked()
                            {
                                *value = Some(SplineCurveConfig {
                                    curve,
                                    min_value: current_min,
                                    max_value: current_max,
                                });
                                changed = true;
                                ui.close();
                            }
                        }
                    });
                }
            });

            // decrease direction submenu
            ui.menu_button(EasingDirection::Decrease.label(), |ui| {
                ui.set_min_width(MENU_WIDTH);
                remove_button_borders(ui);

                // linear option
                if ui
                    .selectable_label(
                        matches!(value, Some(c) if c.curve == SplineCurve::LinearReverse),
                        "Linear",
                    )
                    .clicked()
                {
                    *value = Some(SplineCurveConfig {
                        curve: SplineCurve::LinearReverse,
                        min_value: current_min,
                        max_value: current_max,
                    });
                    changed = true;
                    ui.close();
                }

                for mode in EasingMode::all() {
                    ui.menu_button(mode.label(), |ui| {
                        ui.set_min_width(MENU_WIDTH);
                        remove_button_borders(ui);
                        for easing_type in EasingType::all() {
                            let curve = easing_type.to_curve(EasingDirection::Decrease, *mode);
                            if ui
                                .selectable_label(
                                    matches!(value, Some(c) if c.curve == curve),
                                    easing_type.label(),
                                )
                                .clicked()
                            {
                                *value = Some(SplineCurveConfig {
                                    curve,
                                    min_value: current_min,
                                    max_value: current_max,
                                });
                                changed = true;
                                ui.close();
                            }
                        }
                    });
                }
            });
        });

    changed
}
