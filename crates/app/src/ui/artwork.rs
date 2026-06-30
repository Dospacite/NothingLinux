use adw::prelude::*;

pub(super) fn dot_heading() -> gtk::DrawingArea {
    let area = gtk::DrawingArea::builder()
        .width_request(150)
        .height_request(34)
        .accessible_role(gtk::AccessibleRole::Heading)
        .build();
    area.update_property(&[gtk::accessible::Property::Label("Nothing Linux")]);
    area.set_draw_func(|_, cr, _, _| {
        let patterns = [
            0b11111_u32,
            0b10001,
            0b10001,
            0b10001,
            0b11111,
            0,
            0b10001,
            0b11001,
            0b10101,
            0b10011,
            0b10001,
            0,
            0b11111,
            0b00100,
            0b00100,
            0b00100,
            0b00100,
            0,
            0b10001,
            0b10001,
            0b11111,
            0b10001,
            0b10001,
            0,
            0b10001,
            0b10001,
            0b10101,
            0b11011,
            0b10001,
        ];
        cr.set_source_rgb(0.9, 0.1, 0.12);
        for (row, bits) in patterns.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    cr.arc(
                        5.0 + f64::from(column) * 4.0 + f64::from((row / 6) as u32) * 24.0,
                        7.0 + f64::from((row % 6) as u32) * 4.0,
                        1.2,
                        0.0,
                        std::f64::consts::TAU,
                    );
                    let _ = cr.fill();
                }
            }
        }
    });
    area
}

pub(super) fn draw_earbuds(cr: &gtk::cairo::Context, width: f64, height: f64) {
    let center = width / 2.0;
    cr.set_line_width(10.0);
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    cr.set_source_rgba(0.45, 0.45, 0.48, 0.42);
    cr.move_to(center - 70.0, 58.0);
    cr.curve_to(
        center - 105.0,
        70.0,
        center - 92.0,
        112.0,
        center - 65.0,
        112.0,
    );
    cr.line_to(center - 48.0, height - 38.0);
    let _ = cr.stroke();
    cr.move_to(center + 70.0, 58.0);
    cr.curve_to(
        center + 105.0,
        70.0,
        center + 92.0,
        112.0,
        center + 65.0,
        112.0,
    );
    cr.line_to(center + 48.0, height - 38.0);
    let _ = cr.stroke();
    cr.set_source_rgb(0.9, 0.08, 0.1);
    cr.arc(center - 67.0, 73.0, 5.0, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();
    cr.arc(center + 67.0, 73.0, 5.0, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();
}
