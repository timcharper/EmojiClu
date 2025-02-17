use crate::model::Tile;
use crate::ui::ImageSet;
use gtk4::{prelude::*, IconTheme, TextIter, TextView};
use gtk4::{Box, Label, Orientation, Widget};
use std::rc::Rc;

#[derive(Debug)]
pub enum TemplateElement {
    Label(String),
    Tile(Tile),
    Icon(String),
}

pub struct TemplateParser {
    resources: Rc<ImageSet>,
    icon_theme: Option<Rc<IconTheme>>,
}

impl TemplateParser {
    pub fn new(resources: Rc<ImageSet>, icon_theme: Option<Rc<IconTheme>>) -> Self {
        Self {
            resources,
            icon_theme,
        }
    }

    pub fn parse_template_elements(template: &str) -> Vec<TemplateElement> {
        let mut elements = Vec::new();
        let mut current_text = String::new();
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if !current_text.is_empty() {
                    elements.push(TemplateElement::Label(current_text.clone()));
                    current_text.clear();
                }

                let mut token = String::new();
                while let Some(&next_c) = chars.peek() {
                    chars.next();
                    if next_c == '}' {
                        break;
                    }
                    token.push(next_c);
                }

                if token.starts_with("icon:") {
                    elements.push(TemplateElement::Icon(token[5..].to_string()));
                } else if token.starts_with("tile:") {
                    let tile = Tile::parse(&token[5..]);
                    elements.push(TemplateElement::Tile(tile));
                } else {
                    // probably error out?
                    elements.push(TemplateElement::Label(format!(
                        "error: no such token: {}",
                        token
                    )));
                }
            } else {
                current_text.push(c);
            }
        }

        if !current_text.is_empty() {
            elements.push(TemplateElement::Label(current_text));
        }

        elements
    }

    pub fn parse_as_box(&self, template: &str) -> Box {
        let box_container = Box::new(Orientation::Horizontal, 5);

        TemplateParser::parse_template_elements(template)
            .into_iter()
            .flat_map(|element| match element {
                TemplateElement::Label(text) => {
                    let label = Label::new(None);
                    label.set_markup(&text);
                    label.set_wrap(true);
                    label.set_max_width_chars(40);
                    Some(label.upcast::<Widget>())
                }
                TemplateElement::Tile(tile) => {
                    self.resources.get_candidate_icon(&tile).map(|paintable| {
                        let image = gtk4::Image::from_paintable(Some(paintable.as_ref()));
                        image.upcast::<Widget>()
                    })
                }
                TemplateElement::Icon(icon_name) => {
                    let icon = self.icon_theme.as_ref().unwrap().lookup_icon(
                        &icon_name,
                        &[],
                        24,
                        1,
                        gtk4::TextDirection::Ltr,
                        gtk4::IconLookupFlags::empty(),
                    );
                    let image = gtk4::Image::from_paintable(Some(&icon));
                    Some(image.upcast::<Widget>())
                }
            })
            .for_each(|widget| box_container.append(&widget));

        box_container
    }

    pub fn append_to_text_buffer(
        &self,
        text_view: &TextView,
        pointer: &mut TextIter,
        template: &str,
    ) {
        let buffer = text_view.buffer();
        let elements = TemplateParser::parse_template_elements(template);
        for element in elements {
            match element {
                TemplateElement::Label(text) => {
                    buffer.insert_markup(pointer, &text);
                }
                TemplateElement::Tile(tile) => {
                    let icon = self.resources.get_solution_icon(&tile).unwrap();
                    let image = gtk4::Image::from_paintable(Some(icon.as_ref()));
                    let anchor = buffer.create_child_anchor(pointer);
                    image.set_size_request(32, 32);
                    buffer.insert_child_anchor(pointer, &anchor);
                    text_view.add_child_at_anchor(&image, &anchor);
                    // buffer.insert_paintable(pointer, &image.paintable().unwrap());
                    // buffer.insert_paintable(pointer, icon.as_ref());
                }
                TemplateElement::Icon(icon_name) => {
                    let icon = self.icon_theme.as_ref().unwrap().lookup_icon(
                        &icon_name,
                        &[],
                        32,
                        1,
                        gtk4::TextDirection::Ltr,
                        gtk4::IconLookupFlags::empty(),
                    );
                    let image = gtk4::Image::from_paintable(Some(&icon));
                    let anchor = buffer.create_child_anchor(pointer);
                    buffer.insert_child_anchor(pointer, &anchor);
                    text_view.add_child_at_anchor(&image, &anchor);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_template_elements_with_labels() {
        let template = "This is a {tile:0a} test {tile:1b}";
        let elements = TemplateParser::parse_template_elements(template);

        assert_eq!(elements.len(), 4);
        assert!(matches!(elements[0], TemplateElement::Label(ref text) if text == "This is a "));
        assert!(matches!(elements[1], TemplateElement::Tile(ref tile) if tile.to_string() == "0a"));
        assert!(matches!(elements[2], TemplateElement::Label(ref text) if text == " test "));
        assert!(matches!(elements[3], TemplateElement::Tile(ref tile) if tile.to_string() == "1b"));
    }

    #[test]
    fn test_parse_template_elements_with_only_labels() {
        let template = "Just a label";
        let elements = TemplateParser::parse_template_elements(template);

        assert_eq!(elements.len(), 1);
        assert!(matches!(elements[0], TemplateElement::Label(ref text) if text == "Just a label"));
    }

    #[test]
    fn test_parse_template_elements_with_only_tiles() {
        let template = "{tile:0a}{tile:1b}{tile:2c}";
        let elements = TemplateParser::parse_template_elements(template);

        assert_eq!(elements.len(), 3);
        assert!(matches!(elements[0], TemplateElement::Tile(ref tile) if tile.to_string() == "0a"));
        assert!(matches!(elements[1], TemplateElement::Tile(ref tile) if tile.to_string() == "1b"));
        assert!(matches!(elements[2], TemplateElement::Tile(ref tile) if tile.to_string() == "2c"));
    }

    #[test]
    fn test_parse_template_elements_with_icons() {
        let template = "Test with {icon:view-reveal-symbolic} icon";
        let elements = TemplateParser::parse_template_elements(template);

        assert_eq!(elements.len(), 3);
        assert!(matches!(elements[0], TemplateElement::Label(ref text) if text == "Test with "));
        assert!(
            matches!(elements[1], TemplateElement::Icon(ref name) if name == "view-reveal-symbolic"),
        );
        assert!(matches!(elements[2], TemplateElement::Label(ref text) if text == " icon"));
    }
}
