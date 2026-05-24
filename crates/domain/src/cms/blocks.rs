//! Block discriminated unions для `cms_pages.body`.
//!
//! Top-level: `Block` enum (6 types) — `hero`, `text`, `gallery`, `services`, `cta`,
//! `custom`. Audit §4.3.
//!
//! Real production content (per audit §6.8) использует **только** `custom` блок с
//! `data.ed: Section[]` — это ED-editor tree. Phase A render scope = ED tree only;
//! top-level blocks hero/text/etc. — Phase B (audit C4).
//!
//! ED tree: `Section { id, columns, padding }`, `Column { id, span: f32, elements }`,
//! `CanvasElement` — flat struct с optional fields per widget type (8 widget types).
//!
//! **Critical serde mapping (audit §6.7):**
//! - Widget `type` discriminator `"icon-box"` (с дефисом!) но field name `iconBox` (camelCase)
//! - JSON shape **byte-for-byte** должна совпадать с SITE1; `#[serde(rename = "...")]` обязательно

use serde::{Deserialize, Serialize};

// ─── Top-level Block discriminated union ──────────────────────────────────────

/// Top-level block в `cms_pages.body[]`. 6 variants per audit §1.5 / §4.3.
///
/// Tag = "type", content = "data" per SITE1 schema:
/// ```json
/// { "type": "hero", "data": { "title": "...", "subtitle": "..." } }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum Block {
    Hero(HeroData),
    Text(TextData),
    Gallery(GalleryData),
    Services(ServicesData),
    Cta(CtaData),
    Custom(CustomData),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeroData {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, rename = "imageKey", skip_serializing_if = "Option::is_none")]
    pub image_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextData {
    pub html: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GalleryData {
    #[serde(rename = "mediaIds")]
    pub media_ids: Vec<uuid::Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServicesData {
    #[serde(default, rename = "categoryFilter", skip_serializing_if = "Option::is_none")]
    pub category_filter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CtaData {
    pub label: String,
    pub href: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<CtaStyle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CtaStyle {
    Primary,
    Secondary,
}

/// Custom block — escape hatch с произвольным JSON. Real ED content живёт в
/// `data.ed: Section[]` (audit §6.8).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomData {
    /// ED editor tree (если custom используется ED-editor'ом). Audit §6.8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ed: Option<Vec<Section>>,

    /// Прочие поля, которые могут быть в custom блоке.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

// ─── ED tree: Section / Column / CanvasElement ─────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
    pub columns: Vec<Column>,
    /// CSS padding string (e.g., `"40px 0"`). Audit §6.6 — `<section style="padding: X">`.
    #[serde(default)]
    pub padding: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    /// flex basis weight. Audit §6.6 — `style="flex: span"`. Numeric (не `1|2|3|4|6|12`).
    pub span: f32,
    pub elements: Vec<CanvasElement>,
}

/// Element на canvas'е — flat struct с optional per-widget fields. Audit §4.3.
///
/// `type` discriminator + optional поле соответствующего widget type. Если type='heading',
/// то `heading: Some(HeadingProps)`, остальные None.
///
/// AX serde reads this как flat (для byte-for-byte SITE1 compat). Логика валидации
/// invariant'ов (если type='X' — field 'x' заполнен) — в `Widget::from_canvas_element`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasElement {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: WidgetKind,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<HeadingProps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<TextProps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button: Option<ButtonProps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divider: Option<DividerProps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacer: Option<SpacerProps>,
    #[serde(default, rename = "iconBox", skip_serializing_if = "Option::is_none")]
    pub icon_box: Option<IconBoxProps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cta: Option<CtaProps>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageProps>,

    #[serde(default, rename = "elStyle", skip_serializing_if = "Option::is_none")]
    pub el_style: Option<ElStyle>,
}

/// Widget type discriminator. **Note `IconBox = "icon-box"`** — с дефисом в JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetKind {
    #[serde(rename = "heading")]
    Heading,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "button")]
    Button,
    #[serde(rename = "divider")]
    Divider,
    #[serde(rename = "spacer")]
    Spacer,
    #[serde(rename = "icon-box")]
    IconBox,
    #[serde(rename = "cta")]
    Cta,
    #[serde(rename = "image")]
    Image,
}

// ─── Widget props ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadingProps {
    pub text: String,
    pub tag: HeadingTag,
    pub align: TextAlign,
    pub color: String,
    #[serde(rename = "fontSize")]
    pub font_size: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeadingTag {
    H1,
    H2,
    H3,
    H4,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextProps {
    pub content: String,
    pub align: TextAlign,
    pub color: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ButtonProps {
    pub label: String,
    pub align: TextAlign,
    pub style: ButtonStyle,
    pub size: ButtonSize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Outline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DividerProps {
    #[serde(rename = "lineStyle")]
    pub line_style: DividerLineStyle,
    pub color: String,
    pub weight: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DividerLineStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpacerProps {
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IconBoxProps {
    /// Имя Lucide-иконки (lucide-react key).
    pub icon: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "iconColor")]
    pub icon_color: String,
    pub layout: IconBoxLayout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconBoxLayout {
    Top,
    Left,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CtaProps {
    pub headline: String,
    pub description: String,
    #[serde(rename = "buttonText")]
    pub button_text: String,
    pub align: TextAlign,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageProps {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
}

/// Style overrides per element. Audit §4.3 + §6.6.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElStyle {
    #[serde(rename = "paddingTop")]
    pub padding_top: i32,
    #[serde(rename = "paddingRight")]
    pub padding_right: i32,
    #[serde(rename = "paddingBottom")]
    pub padding_bottom: i32,
    #[serde(rename = "paddingLeft")]
    pub padding_left: i32,
    pub background: String,
    #[serde(rename = "borderRadius")]
    pub border_radius: i32,
    pub opacity: u32,
    /// M1: editor hides/ignores это поле. Audit §4.3 + ed-types.ts comment.
    #[serde(rename = "customCss", default)]
    pub custom_css: String,
}

impl ElStyle {
    /// Default per `defaultElStyle()` в SITE1 ed-types.ts:98-107.
    #[must_use]
    pub fn default_values() -> Self {
        Self {
            padding_top: 12,
            padding_right: 12,
            padding_bottom: 12,
            padding_left: 12,
            background: "transparent".to_string(),
            border_radius: 0,
            opacity: 100,
            custom_css: String::new(),
        }
    }
}

// ─── Widget — typed enum (для convenience render code) ─────────────────────────

/// Typed enum для widget — convenience wrapper над `CanvasElement` flat struct.
///
/// `CanvasElement` сериализуется в byte-for-byte SITE1 JSON; `Widget` enum используется
/// внутри render логики для exhaustive `match`.
#[derive(Clone, Debug, PartialEq)]
pub enum Widget {
    Heading(HeadingProps),
    Text(TextProps),
    Button(ButtonProps),
    Divider(DividerProps),
    Spacer(SpacerProps),
    IconBox(IconBoxProps),
    Cta(CtaProps),
    Image(ImageProps),
}

impl Widget {
    /// Try конвертация из `CanvasElement` (flat) → `Widget` (typed). None if mismatched.
    #[must_use]
    pub fn from_canvas_element(el: &CanvasElement) -> Option<Self> {
        match el.kind {
            WidgetKind::Heading => el.heading.clone().map(Self::Heading),
            WidgetKind::Text => el.text.clone().map(Self::Text),
            WidgetKind::Button => el.button.clone().map(Self::Button),
            WidgetKind::Divider => el.divider.clone().map(Self::Divider),
            WidgetKind::Spacer => el.spacer.clone().map(Self::Spacer),
            WidgetKind::IconBox => el.icon_box.clone().map(Self::IconBox),
            WidgetKind::Cta => el.cta.clone().map(Self::Cta),
            WidgetKind::Image => el.image.clone().map(Self::Image),
        }
    }
}

/// Extract ED tree из `cms_pages.body[]`. Аналог `extractEdSections` (audit §6.8).
/// Возвращает первый `Block::Custom` с непустым `ed`. Иначе `vec![]`.
#[must_use]
pub fn extract_ed_sections(body: &[Block]) -> Vec<Section> {
    for block in body {
        if let Block::Custom(custom) = block {
            if let Some(ed) = &custom.ed {
                return ed.clone();
            }
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_hero_serde_roundtrip() {
        let block = Block::Hero(HeroData {
            title: "Welcome".into(),
            subtitle: Some("To our spa".into()),
            image_key: None,
        });
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "hero");
        assert_eq!(json["data"]["title"], "Welcome");
        assert_eq!(json["data"]["subtitle"], "To our spa");
        assert!(json["data"].get("imageKey").is_none());

        let parsed: Block = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, block);
    }

    #[test]
    fn block_text_serde() {
        let block = Block::Text(TextData {
            html: "<p>Hello</p>".into(),
        });
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["data"]["html"], "<p>Hello</p>");
    }

    #[test]
    fn widget_kind_icon_box_serializes_with_hyphen() {
        let json = serde_json::to_string(&WidgetKind::IconBox).unwrap();
        assert_eq!(json, r#""icon-box""#);
    }

    #[test]
    fn canvas_element_round_trip_heading() {
        let el = CanvasElement {
            id: "el-1".into(),
            kind: WidgetKind::Heading,
            heading: Some(HeadingProps {
                text: "Title".into(),
                tag: HeadingTag::H1,
                align: TextAlign::Center,
                color: "#FFFFFF".into(),
                font_size: 48,
            }),
            text: None,
            button: None,
            divider: None,
            spacer: None,
            icon_box: None,
            cta: None,
            image: None,
            el_style: None,
        };
        let json = serde_json::to_value(&el).unwrap();
        assert_eq!(json["type"], "heading");
        assert_eq!(json["heading"]["text"], "Title");
        assert_eq!(json["heading"]["fontSize"], 48);
        assert!(json.get("text").is_none()); // skip_serializing_if

        let parsed: CanvasElement = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, el);
    }

    #[test]
    fn canvas_element_icon_box_uses_icon_box_field_name() {
        let el = CanvasElement {
            id: "el-2".into(),
            kind: WidgetKind::IconBox,
            heading: None,
            text: None,
            button: None,
            divider: None,
            spacer: None,
            icon_box: Some(IconBoxProps {
                icon: "Heart".into(),
                title: "Care".into(),
                description: "We care".into(),
                icon_color: "#D4AF37".into(),
                layout: IconBoxLayout::Top,
            }),
            cta: None,
            image: None,
            el_style: None,
        };
        let json = serde_json::to_value(&el).unwrap();
        assert_eq!(json["type"], "icon-box");
        // Field name "iconBox" (camelCase), NOT "icon-box"
        assert_eq!(json["iconBox"]["icon"], "Heart");
        assert_eq!(json["iconBox"]["iconColor"], "#D4AF37");
    }

    #[test]
    fn section_with_columns_and_widgets() {
        let section = Section {
            id: "sec-1".into(),
            columns: vec![Column {
                id: "col-1".into(),
                span: 6.0,
                elements: vec![],
            }],
            padding: "40px 0".into(),
        };
        let json = serde_json::to_value(&section).unwrap();
        assert_eq!(json["id"], "sec-1");
        assert_eq!(json["columns"][0]["span"], 6.0);
        assert_eq!(json["padding"], "40px 0");
    }

    #[test]
    fn extract_ed_sections_finds_custom_block_ed() {
        let sections = vec![Section {
            id: "s1".into(),
            columns: vec![],
            padding: String::new(),
        }];
        let body = vec![
            Block::Hero(HeroData {
                title: "x".into(),
                subtitle: None,
                image_key: None,
            }),
            Block::Custom(CustomData {
                ed: Some(sections.clone()),
                extra: serde_json::Map::new(),
            }),
        ];
        assert_eq!(extract_ed_sections(&body), sections);
    }

    #[test]
    fn extract_ed_sections_returns_empty_if_no_custom() {
        let body = vec![Block::Text(TextData {
            html: "x".into(),
        })];
        assert!(extract_ed_sections(&body).is_empty());
    }

    #[test]
    fn extract_ed_sections_empty_if_custom_has_no_ed() {
        let body = vec![Block::Custom(CustomData {
            ed: None,
            extra: serde_json::Map::new(),
        })];
        assert!(extract_ed_sections(&body).is_empty());
    }

    #[test]
    fn el_style_defaults_match_site1() {
        let s = ElStyle::default_values();
        assert_eq!(s.padding_top, 12);
        assert_eq!(s.background, "transparent");
        assert_eq!(s.opacity, 100);
        assert!(s.custom_css.is_empty());
    }
}
