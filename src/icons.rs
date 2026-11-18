use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Home,
    Globe,
    Shield,
    Info,
    Search,
    Bolt,
    Check,
    Close,
    Reset,
    Plus,
}

pub fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32, size_units: f32) {
    let side = rect.width().min(rect.height());
    let origin = rect.center() - Vec2::splat(side * 0.5);
    let point = |x: f32, y: f32| origin + Vec2::new(x * side, y * side);
    let unit = side / 24.0;
    let stroke = Stroke::new((size_units * unit).max(1.0), color);
    let thin = Stroke::new((size_units * unit * 0.82).max(0.9), color);

    match icon {
        Icon::Home => {
            painter.add(Shape::line(
                vec![point(0.14, 0.48), point(0.5, 0.17), point(0.86, 0.48)],
                stroke,
            ));
            painter.add(Shape::line(
                vec![
                    point(0.23, 0.44),
                    point(0.23, 0.84),
                    point(0.77, 0.84),
                    point(0.77, 0.44),
                ],
                stroke,
            ));
            painter.add(Shape::line(
                vec![
                    point(0.42, 0.84),
                    point(0.42, 0.62),
                    point(0.58, 0.62),
                    point(0.58, 0.84),
                ],
                thin,
            ));
        }
        Icon::Globe => {
            painter.circle_stroke(rect.center(), side * 0.34, stroke);
            painter.add(Shape::line(
                ellipse(
                    rect.center(),
                    side * 0.34,
                    side * 0.34 * 0.42,
                    0.0,
                    std::f32::consts::TAU,
                    36,
                ),
                thin,
            ));
            painter.add(Shape::line(
                ellipse(
                    rect.center(),
                    side * 0.34 * 0.42,
                    side * 0.34,
                    0.0,
                    std::f32::consts::TAU,
                    36,
                ),
                thin,
            ));
        }
        Icon::Shield => {
            let points = vec![
                point(0.5, 0.13),
                point(0.84, 0.27),
                point(0.84, 0.52),
                point(0.5, 0.88),
                point(0.16, 0.52),
                point(0.16, 0.27),
                point(0.5, 0.13),
            ];
            painter.add(Shape::line(points, stroke));
        }
        Icon::Info => {
            painter.circle_stroke(rect.center(), side * 0.34, stroke);
            painter.circle_filled(point(0.5, 0.32), unit * 1.2, color);
            painter.add(Shape::line(
                vec![point(0.5, 0.47), point(0.5, 0.72)],
                stroke,
            ));
        }
        Icon::Search => {
            painter.circle_stroke(point(0.44, 0.44), side * 0.26, stroke);
            painter.add(Shape::line(
                vec![point(0.64, 0.64), point(0.84, 0.84)],
                stroke,
            ));
        }
        Icon::Bolt => {
            painter.add(Shape::convex_polygon(
                vec![
                    point(0.57, 0.08),
                    point(0.24, 0.56),
                    point(0.46, 0.56),
                    point(0.42, 0.92),
                    point(0.77, 0.42),
                    point(0.55, 0.42),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Check => {
            painter.add(Shape::line(
                vec![point(0.18, 0.52), point(0.41, 0.74), point(0.83, 0.26)],
                Stroke::new(stroke.width * 1.1, color),
            ));
        }
        Icon::Close => {
            painter.add(Shape::line(
                vec![point(0.25, 0.25), point(0.75, 0.75)],
                stroke,
            ));
            painter.add(Shape::line(
                vec![point(0.75, 0.25), point(0.25, 0.75)],
                stroke,
            ));
        }
        Icon::Reset => {
            let c = rect.center();
            let r = side * 0.33;
            painter.add(Shape::line(
                ellipse(
                    c,
                    r,
                    r,
                    -std::f32::consts::FRAC_PI_2 + 0.45,
                    std::f32::consts::TAU - 0.75,
                    32,
                ),
                stroke,
            ));
            let start = angle_point(c, r, -std::f32::consts::FRAC_PI_2 + 0.45);
            painter.add(Shape::convex_polygon(
                vec![
                    start + Vec2::new(-unit * 1.4, unit * 1.1),
                    start + Vec2::new(unit * 2.2, unit * 1.6),
                    start + Vec2::new(unit * 0.2, unit * 3.4),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Plus => {
            painter.add(Shape::line(
                vec![point(0.5, 0.24), point(0.5, 0.76)],
                stroke,
            ));
            painter.add(Shape::line(
                vec![point(0.24, 0.5), point(0.76, 0.5)],
                stroke,
            ));
        }
    }
}

fn angle_point(center: Pos2, radius: f32, angle: f32) -> Pos2 {
    center + Vec2::new(angle.cos() * radius, angle.sin() * radius)
}

fn ellipse(center: Pos2, rx: f32, ry: f32, start: f32, end: f32, steps: usize) -> Vec<Pos2> {
    (0..=steps)
        .map(|i| {
            let t = start + (end - start) * (i as f32 / steps as f32);
            center + Vec2::new(t.cos() * rx, t.sin() * ry)
        })
        .collect()
}
