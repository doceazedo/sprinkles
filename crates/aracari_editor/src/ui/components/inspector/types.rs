use crate::ui::widgets::vector_edit::VectorSuffixes;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FieldKind {
    #[default]
    F32,
    F32Percent,
    U32,
    U32OrEmpty,
    OptionalU32,
    Bool,
    Vector(VectorSuffixes),
    ComboBox { options: Vec<String> },
    Color,
    Gradient,
    Curve,
}

#[derive(Debug, Clone, Default)]
pub struct FieldDef {
    pub name: String,
    pub kind: FieldKind,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub suffix: Option<String>,
    pub placeholder: Option<String>,
    pub min: Option<f32>,
    pub max: Option<f32>,
}

impl FieldDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn f32(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::F32)
    }

    pub fn f32_percent(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::F32Percent)
    }

    pub fn u32(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::U32)
    }

    pub fn u32_or_empty(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::U32OrEmpty)
    }

    pub fn optional_u32(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::OptionalU32)
    }

    pub fn bool(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::Bool)
    }

    pub fn vector(name: impl Into<String>, suffixes: VectorSuffixes) -> Self {
        Self::new(name).with_kind(FieldKind::Vector(suffixes))
    }

    pub fn combobox(name: impl Into<String>, options: Vec<impl Into<String>>) -> Self {
        Self::new(name).with_kind(FieldKind::ComboBox {
            options: options.into_iter().map(Into::into).collect(),
        })
    }

    pub fn color(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::Color)
    }

    pub fn gradient(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::Gradient)
    }

    pub fn with_kind(mut self, kind: FieldKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn with_min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    pub fn with_max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }
}
