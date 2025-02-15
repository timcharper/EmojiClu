use crate::model::Tile;
use crate::ui::ImageSet;
use gtk4::prelude::*;
use gtk4::{Box, Label, Orientation, Widget};
use std::rc::Rc;

#[derive(Debug)]
pub enum TemplateElement {
    Label(String),
    Tile(Tile),
}

pub struct TemplateParser {
    resources: Rc<ImageSet>,
}

impl TemplateParser {
    pub fn new(resources: Rc<ImageSet>) -> Self {
        Self { resources }
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

                let tile = Tile::parse(&token);
                elements.push(TemplateElement::Tile(tile));
            } else {
                current_text.push(c);
            }
        }

        if !current_text.is_empty() {
            elements.push(TemplateElement::Label(current_text));
        }

        elements
    }

    pub fn parse_template(&self, template: &str) -> Box {
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
            })
            .for_each(|widget| box_container.append(&widget));

        box_container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_template_elements_with_labels() {
        let template = "This is a {0a} test {1b}";
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
        let template = "{0a}{1b}{2c}";
        let elements = TemplateParser::parse_template_elements(template);

        assert_eq!(elements.len(), 3);
        assert!(matches!(elements[0], TemplateElement::Tile(ref tile) if tile.to_string() == "0a"));
        assert!(matches!(elements[1], TemplateElement::Tile(ref tile) if tile.to_string() == "1b"));
        assert!(matches!(elements[2], TemplateElement::Tile(ref tile) if tile.to_string() == "2c"));
    }
}
