//! range attributes should override default attributes

use crate::kurbo::{Size, Vec2};
use crate::{
    Color, Error, FontFamily, FontWeight, RenderContext, Text, TextAttribute, TextLayout,
    TextLayoutBuilder,
};

pub const SIZE: Size = Size::new(480., 560.);

static TEXT: &str = r#"Philosophers often behave like little children who scribble some marks on a piece of paper at random and then ask the grown-up "What's that?" — It happened like this: the grown-up had drawn pictures for the child several times and said "this is a man," "this is a house," etc. And then the child makes some marks too and asks: what's this then?"#;

const RED: Color = Color::rgb8(255, 0, 0);
const BLUE: Color = Color::rgb8(0, 0, 255);

pub fn draw<R: RenderContext>(rc: &mut R) -> Result<(), Error> {
    rc.clear(Color::WHITE);
    let text = rc.text();
    //let _ = text
    //.load_font(include_bytes!("../../../resources/Anaheim-Regular.ttf"))
    //.unwrap();
    //let font = text
    //.load_font(include_bytes!("../../../resources/Anaheim-Bold.ttf"))
    //.unwrap();
    //let font2 = text.font_family("Alfa Slab One").unwrap();

    let font = FontFamily::SYSTEM_UI;
    let font2 = text.font_family("Courier New").unwrap();
    let layout = text
        .new_text_layout(TEXT)
        .max_width(200.0)
        .default_attribute(font2)
        .default_attribute(TextAttribute::Underline(true))
        .default_attribute(TextAttribute::Italic(true))
        .default_attribute(TextAttribute::ForegroundColor(RED))
        .default_attribute(FontWeight::BOLD)
        .range_attribute(..200, TextAttribute::ForegroundColor(BLUE))
        .range_attribute(10..100, FontWeight::NORMAL)
        .range_attribute(40..300, TextAttribute::Underline(false))
        .range_attribute(60..160, TextAttribute::Italic(false))
        .range_attribute(140..220, FontWeight::NORMAL)
        .range_attribute(200.., font)
        .range_attribute(240..260, TextAttribute::Italic(true))
        .build()?;

    let y_pos = ((SIZE.height - layout.size().height * 2.0) / 4.0).max(0.0);
    let text_pos = Vec2::new(16.0, y_pos);
    rc.draw_text(&layout, text_pos.to_point());

    Ok(())
}
