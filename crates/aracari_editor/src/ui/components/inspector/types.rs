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
    AnimatedVelocity,
    TextureRef,
}

#[derive(Debug, Clone, Default)]
pub struct VariantField {
    pub name: String,
    pub kind: FieldKind,
}

impl VariantField {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: FieldKind::default(),
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

    pub fn animated_velocity(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::AnimatedVelocity)
    }

    pub fn texture_ref(name: impl Into<String>) -> Self {
        Self::new(name).with_kind(FieldKind::TextureRef)
    }

    pub fn with_kind(mut self, kind: FieldKind) -> Self {
        self.kind = kind;
        self
    }
}
