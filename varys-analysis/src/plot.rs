use plotters::coord::Shift;
use std::path::Path;

use plotters::prelude::*;
use plotters::style::SizeDesc;

use crate::ml::data::NumericTraceDataset;
use crate::trace::NumericTrafficTrace;

const MAX_VALUE: i32 = 1514;

pub fn plot_queries<P: AsRef<Path>>(
    data_path: P,
    queries: Vec<&str>,
    dataset: &NumericTraceDataset,
) {
    for query in queries {
        plot(data_path.as_ref(), query, dataset);
    }
}

pub fn plot<P: AsRef<Path>>(data_path: P, query: &str, dataset: &NumericTraceDataset) {
    let path = data_path.as_ref().join(format!("plots/plot-{query}.png"));

    let label = dataset.get_label(query).unwrap();
    let mut traces = dataset.items.iter().filter(|item| item.label == label);

    let drawing_area = BitMapBackend::new(&path, (800, 1400)).into_drawing_area();
    drawing_area.fill(&WHITE).unwrap();
    let areas = drawing_area.split_evenly((100, 1));

    for area in areas {
        plot_trace(&traces.next().unwrap().trace, &area, false, (0, 0, 0, 0));
    }
}

fn plot_trace<DB: DrawingBackend, S: SizeDesc>(
    trace: &NumericTrafficTrace,
    drawing_area: &DrawingArea<DB, Shift>,
    show_mesh: bool,
    margin: (S, S, S, S),
) {
    let data = &trace.0;
    let mut chart = ChartBuilder::on(drawing_area)
        .margin_top(margin.0)
        .margin_right(margin.1)
        .margin_bottom(margin.2)
        .margin_left(margin.3)
        .build_cartesian_2d(0..data.len() as i32, -MAX_VALUE..(MAX_VALUE + 1))
        .unwrap();
    if show_mesh {
        chart
            .configure_mesh()
            .light_line_style(TRANSPARENT)
            .bold_line_style(RGBAColor(0, 0, 0, 0.2))
            .draw()
            .unwrap();
    }

    chart
        .draw_series(data.iter().enumerate().map(|(x, &value)| {
            let x = x as i32;
            let size = MAX_VALUE / 2 + (value / 2.).round().abs() as i32;
            let style = if value.abs() < 0.001 {
                TRANSPARENT.filled()
            } else {
                color(value as f64).filled()
            };

            Rectangle::new([(x, -size), (x + 1, size)], style)
        }))
        .unwrap();
}

fn color(value: f64) -> HSLColor {
    let scaled = value / MAX_VALUE as f64;

    HSLColor(
        if scaled >= 0. { hue(215.) } else { hue(15.) },
        1.,
        0.4 + 0.6 * (1. - scaled.abs()).powi(2),
    )
}

#[inline]
fn hue(value: f64) -> f64 {
    value / 360.
}
