use crate::ui::widgets::vector_edit::VectorSuffixes;

#[derive(Debug, Clone, PartialEq)]
pub struct ComboBoxOption {
    pub label: String,
    pub value: String,
}

impl ComboBoxOption {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FieldKind {
    #[default]
    F32,
    F32Percent,
    F32OrInfinity,
    U32,
    U32OrEmpty,
    OptionalU32,
    Bool,
    Vector(VectorSuffixes),
    ComboBox {
        options: Vec<ComboBoxOption>,
        optional: bool,
    },
    Color,
    Gradient,
    Curve,
    AnimatedVelocity,
    TextureRef,
}

#[derive(Debug, Clone, Default)]
pub struct VariantField {
    pub name: String,
    pub kind: FieldKind,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl VariantField {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: FieldKind::default(),
            min: None,
            max: None,
        }
    }

    pub fn f32(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::F32)
    }

    pub fn u32(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::U32)
    }

    pub fn bool(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::Bool)
    }

    pub fn vector(name: impl Into<String>, suffixes: VectorSuffixes) -> Self {
        Self::new(name).with_kind(FieldKind::Vector(suffixes))
    }

    pub fn combobox(name: impl Into<String>, options: Vec<ComboBoxOption>) -> Self {
        Self::new(name).with_kind(FieldKind::ComboBox {
            options,
            optional: false,
        })
    }

    pub fn optional_combobox(name: impl Into<String>, options: Vec<ComboBoxOption>) -> Self {
        Self::new(name).with_kind(FieldKind::ComboBox {
            options,
            optional: true,
        })
    }

    pub fn color(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::Color)
    }

    pub fn gradient(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::Gradient)
    }

    pub fn animated_velocity(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::AnimatedVelocity)
    }

    pub fn texture_ref(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::TextureRef)
    }

    pub fn percent(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::F32Percent)
    }

    pub fn with_kind(mut self, kind: FieldKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    pub fn with_max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }
}
