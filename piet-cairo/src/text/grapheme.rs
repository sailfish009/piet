use cairo::ScaledFont;
use piet::HitTestPoint;
use unicode_segmentation::UnicodeSegmentation;

use super::hit_test_line_position;

/// get grapheme boundaries, intended to act on a line of text, not a full text layout that has
/// both horizontal and vertial components
pub(crate) fn get_grapheme_boundaries(
    font: &ScaledFont,
    text: &str,
    grapheme_position: usize,
) -> Option<GraphemeBoundaries> {
    let mut graphemes = UnicodeSegmentation::grapheme_indices(text, true);
    let (text_position, _) = graphemes.nth(grapheme_position)?;
    let (next_text_position, _) = graphemes.next().unwrap_or_else(|| (text.len(), ""));

    let curr_edge = hit_test_line_position(font, text, text_position)?;
    let next_edge = hit_test_line_position(font, text, next_text_position)?;

    let res = GraphemeBoundaries {
        curr_idx: text_position,
        next_idx: next_text_position,
        leading: curr_edge,
        trailing: next_edge,
    };

    Some(res)
}

pub(crate) fn point_x_in_grapheme(
    point_x: f64,
    grapheme_boundaries: &GraphemeBoundaries,
) -> Option<HitTestPoint> {
    let leading = grapheme_boundaries.leading;
    let trailing = grapheme_boundaries.trailing;
    let curr_idx = grapheme_boundaries.curr_idx;
    let next_idx = grapheme_boundaries.next_idx;

    if point_x >= leading && point_x <= trailing {
        // Check which boundary it's closer to.
        // Round up to next grapheme boundary if
        let midpoint = leading + ((trailing - leading) / 2.0);
        let is_inside = true;
        let idx = if point_x >= midpoint {
            next_idx
        } else {
            curr_idx
        };
        Some(HitTestPoint::new(idx, is_inside))
    } else {
        None
    }
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct GraphemeBoundaries {
    pub curr_idx: usize,
    pub next_idx: usize,
    pub leading: f64,
    // not technically trailing; it's the lead boundary for the next grapheme cluster
    pub trailing: f64,
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::text::*;

    #[test]
    fn test_grapheme_boundaries() {
        let text = "piet";

        let font = CairoFont::new(FontFamily::SANS_SERIF).resolve_simple(12.0);

        let expected_3 = GraphemeBoundaries {
            curr_idx: 3,
            next_idx: 4,
            leading: 0.0,  // not testing x advance
            trailing: 0.0, // not testing x advance
        };

        // test grapheme boundaries
        assert_eq!(
            get_grapheme_boundaries(&font, text, 3).unwrap().curr_idx,
            expected_3.curr_idx
        );
        assert_eq!(
            get_grapheme_boundaries(&font, text, 3).unwrap().next_idx,
            expected_3.next_idx
        );
        assert_eq!(get_grapheme_boundaries(&font, text, 4), None);
    }

    #[test]
    fn test_x_in_grapheme_boundaries() {
        let bounds = GraphemeBoundaries {
            curr_idx: 2,
            next_idx: 4,
            leading: 10.0,
            trailing: 14.0,
        };

        let expected_curr = Some(HitTestPoint::new(2, true));
        let expected_next = Some(HitTestPoint::new(4, true));

        assert_eq!(point_x_in_grapheme(10.0, &bounds), expected_curr);
        assert_eq!(point_x_in_grapheme(11.0, &bounds), expected_curr);
        assert_eq!(point_x_in_grapheme(12.0, &bounds), expected_next);
        assert_eq!(point_x_in_grapheme(13.0, &bounds), expected_next);
    }
}
