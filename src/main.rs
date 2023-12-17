use std::sync::Mutex;

use cairo::Context;
use gtk::{glib, Application, ApplicationWindow, DrawingArea};
use gtk::{prelude::*, Button, Grid, ToggleButton};
use once_cell::sync::Lazy;

const APP_ID: &str = "org.gtk_rs.HelloWorld2";

fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

const JOINT_RADIUS: f64 = 10.0;
const STROKE_WIDTH: f64 = 1.5;
const SUPPORT_TRIANGLE_WIDTH: f64 = 50.0;
const SUPPORT_TRIANGLE_HEIGHT: f64 = 30.0;
const SUPPORT_BASE_WIDTH: f64 = 70.0;
const SUPPORT_LINE_HEIGHT: f64 = 20.0;
const SUPPORT_LINE_WIDTH: f64 = 5.0;
const SUPPORT_LINE_MARGIN: f64 = 1.0;
const SUPPORT_LINE_COUNT: usize = 5;
const COUPLER_CURVE_RESOLUTION: usize = 1000;

static FOUR_BAR: Lazy<Mutex<[(f64, f64); 5]>> = Lazy::new(|| {
    Mutex::new([
        (550.0, 350.0),
        (300.0, 400.0),
        (350.0, 550.0),
        (600.0, 600.0),
        (440.0, 550.0),
    ])
});
static SELECTED_JOINT: Lazy<Mutex<Option<(usize, (f64, f64))>>> = Lazy::new(|| Mutex::new(None));
static ANIMATE: Lazy<Mutex<Option<[(f64, f64); 5]>>> = Lazy::new(|| Mutex::new(None));

fn draw_support(context: &Context, p: (f64, f64)) {
    context.save();
    context.set_line_width(STROKE_WIDTH);

    context.arc(p.0, p.1, JOINT_RADIUS, 0.0, 2.0 * std::f64::consts::PI);
    context.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    context.fill_preserve();
    context.set_source_rgba(0.0, 0.0, 0.0, 0.6);
    context.stroke();

    context.move_to(
        p.0 - JOINT_RADIUS / 2.0_f64.sqrt(),
        p.1 + JOINT_RADIUS / 2.0_f64.sqrt(),
    );
    context.line_to(
        p.0 - SUPPORT_TRIANGLE_WIDTH / 2.0,
        p.1 + SUPPORT_TRIANGLE_HEIGHT,
    );

    context.move_to(
        p.0 + JOINT_RADIUS / 2.0_f64.sqrt(),
        p.1 + JOINT_RADIUS / 2.0_f64.sqrt(),
    );
    context.line_to(
        p.0 + SUPPORT_TRIANGLE_WIDTH / 2.0,
        p.1 + SUPPORT_TRIANGLE_HEIGHT,
    );

    context.move_to(
        p.0 - SUPPORT_BASE_WIDTH / 2.0,
        p.1 + SUPPORT_TRIANGLE_HEIGHT,
    );
    context.line_to(
        p.0 + SUPPORT_BASE_WIDTH / 2.0,
        p.1 + SUPPORT_TRIANGLE_HEIGHT,
    );

    for i in 0..SUPPORT_LINE_COUNT {
        context.move_to(
            p.0 - SUPPORT_BASE_WIDTH / 2.0
                + SUPPORT_LINE_MARGIN
                + (i as f64 / SUPPORT_LINE_COUNT as f64)
                    * (SUPPORT_BASE_WIDTH - SUPPORT_LINE_MARGIN),
            p.1 + SUPPORT_TRIANGLE_HEIGHT + SUPPORT_LINE_HEIGHT,
        );
        context.rel_line_to(SUPPORT_LINE_WIDTH, -SUPPORT_LINE_HEIGHT);
    }

    context.stroke();
    context.restore();
}

fn draw_joint(context: &Context, p: (f64, f64)) {
    context.save();

    context.set_line_width(STROKE_WIDTH);
    context.arc(p.0, p.1, JOINT_RADIUS, 0.0, 2.0 * std::f64::consts::PI);
    context.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    context.fill_preserve();
    context.set_source_rgba(0.0, 0.0, 0.0, 0.6);
    context.stroke();

    context.restore();
}

fn draw_connecting_line(context: &Context, p1: (f64, f64), p2: (f64, f64)) {
    context.save();

    context.set_line_width(STROKE_WIDTH);
    context.set_source_rgba(0.0, 0.0, 0.0, 0.6);
    context.move_to(p1.0, p1.1);
    context.line_to(p2.0, p2.1);
    context.stroke();

    context.restore();
}

fn draw_four_bar_linkage(context: &Context, joints: [(f64, f64); 5]) {
    for i in 0..4 {
        draw_connecting_line(context, joints[i], joints[(i + 1) % 4]);
    }

    draw_coupler_curve(context, joints);

    draw_joint(context, joints[0]);
    draw_joint(context, joints[1]);
    draw_support(context, joints[2]);
    draw_support(context, joints[3]);
}

fn length(p1: (f64, f64), p2: (f64, f64)) -> f64 {
    ((p2.0 - p1.0).powf(2.0) + (p2.1 - p1.1).powf(2.0)).sqrt()
}

fn angle(p1: (f64, f64), p2: (f64, f64)) -> f64 {
    -(p1.1 - p2.1).atan2(p1.0 - p2.0)
}

fn get_rocker_pos(p1: (f64, f64), joints: [(f64, f64); 5]) -> (f64, f64) {
    let crank_length = length(joints[1], joints[2]);
    let rocker_length = length(joints[0], joints[3]);
    let coupler_length = length(joints[0], joints[1]);

    let p2 = (joints[3].0, joints[3].1);
    let r = length(p1, p2);
    (
        0.5 * (p1.0 + p2.0)
            + (coupler_length.powf(2.0) - rocker_length.powf(2.0)) / (2.0 * r.powf(2.0))
                * (p2.0 - p1.0)
            + 0.5
                * (2.0 * (coupler_length.powf(2.0) + rocker_length.powf(2.0)) / r.powf(2.0)
                    - (coupler_length.powf(2.0) - rocker_length.powf(2.0)).powf(2.0) / r.powf(4.0)
                    - 1.0)
                    .sqrt()
                * (p2.1 - p1.1),
        0.5 * (p1.1 + p2.1)
            + (coupler_length.powf(2.0) - rocker_length.powf(2.0)) / (2.0 * r.powf(2.0))
                * (p2.1 - p1.1)
            + 0.5
                * (2.0 * (coupler_length.powf(2.0) + rocker_length.powf(2.0)) / r.powf(2.0)
                    - (coupler_length.powf(2.0) - rocker_length.powf(2.0)).powf(2.0) / r.powf(4.0)
                    - 1.0)
                    .sqrt()
                * (p1.0 - p2.0),
    )
}

fn draw_coupler_curve(context: &Context, joints: [(f64, f64); 5]) {
    context.save();

    context.set_line_width(1.5 * STROKE_WIDTH);
    context.set_source_rgba(1.0, 0.0, 0.0, 0.6);

    context.move_to(joints[4].0, joints[4].1);

    let theta = angle(joints[0], joints[1]);
    let (dx, dy) = (joints[4].0 - joints[1].0, joints[4].1 - joints[1].1);
    let crank_length = length(joints[1], joints[2]);

    for i in 0..COUPLER_CURVE_RESOLUTION {
        let a = std::f64::consts::PI * 2.0 * i as f64 / COUPLER_CURVE_RESOLUTION as f64
            + angle(joints[1], joints[2]);
        let p1 = (
            joints[2].0 + crank_length * a.cos(),
            joints[2].1 - crank_length * a.sin(),
        );
        let p3 = get_rocker_pos(p1, joints);

        let psi = angle(p3, p1);
        let p4 = (
            p1.0 + dx * (theta - psi).cos() - dy * (theta - psi).sin(),
            p1.1 + dy * (theta - psi).cos() + dx * (theta - psi).sin(),
        );
        context.line_to(p4.0, p4.1);
        context.stroke();
        context.move_to(p4.0, p4.1);
    }

    context.line_to(joints[4].0, joints[4].1);
    context.stroke();

    context.set_line_width(STROKE_WIDTH);

    context.set_source_rgba(0.0, 0.0, 0.0, 0.6);
    context.move_to(joints[0].0, joints[0].1);
    context.line_to(joints[4].0, joints[4].1);
    context.line_to(joints[1].0, joints[1].1);
    context.stroke();

    context.arc(
        joints[4].0,
        joints[4].1,
        5.0,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    context.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    context.fill_preserve();
    context.set_source_rgba(0.0, 0.0, 0.0, 0.6);
    context.stroke();

    context.restore();
}

fn build_ui(app: &Application) {
    // Create a button with label and margins
    let drawing_area = DrawingArea::builder()
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .content_height(1000)
        .content_width(1000)
        .build();

    drawing_area.set_draw_func(|area, context, width, height| {
        draw_four_bar_linkage(context, *FOUR_BAR.lock().unwrap());
    });

    let gesture = gtk::GestureDrag::new();
    gesture.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
    gesture.connect_drag_begin(|_, x, y| {
        if ANIMATE.lock().unwrap().is_some() {
            return;
        }

        for (i, p) in FOUR_BAR.lock().unwrap().iter().enumerate() {
            if length(*p, (x, y)) < (JOINT_RADIUS + 10.0) {
                // selecting current joint
                *SELECTED_JOINT.lock().unwrap() = Some((i, *p));
                return;
            }
        }
        *SELECTED_JOINT.lock().unwrap() = None;
    });
    gesture.connect_drag_update(|gesture, x, y| {
        if ANIMATE.lock().unwrap().is_some() {
            return;
        }

        if let Some((joint, p)) = *SELECTED_JOINT.lock().unwrap() {
            FOUR_BAR.lock().unwrap()[joint] = (p.0 + x, p.1 + y);
            gesture.widget().queue_draw();
        }
    });
    gesture.connect_drag_end(|_, _, _| {
        *SELECTED_JOINT.lock().unwrap() = None;
    });

    drawing_area.add_controller(gesture);

    let button = ToggleButton::builder().label("Animate").build();
    button.connect_toggled(|button| {
        if button.is_active() {
            *ANIMATE.lock().unwrap() = Some(*FOUR_BAR.lock().unwrap());
        } else {
            *FOUR_BAR.lock().unwrap() = ANIMATE.lock().unwrap().unwrap();
            *ANIMATE.lock().unwrap() = None;
        }
    });

    let grid = Grid::builder().row_spacing(10).build();
    grid.attach(&drawing_area, 0, 0, 1, 1);
    grid.attach(&button, 0, 1, 1, 1);

    // Create a window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My GTK App")
        .child(&grid)
        .build();

    window.add_tick_callback(|window, _| {
        if ANIMATE.lock().unwrap().is_some() {
            let mut joints = FOUR_BAR.lock().unwrap();

            let crank_length = length(joints[1], joints[2]);
            let theta = angle(joints[0], joints[1]);
            let (dx, dy) = (joints[4].0 - joints[1].0, joints[4].1 - joints[1].1);

            let a = 0.02 + angle(joints[1], joints[2]);
            let p1 = (
                joints[2].0 + crank_length * a.cos(),
                joints[2].1 - crank_length * a.sin(),
            );
            let p3 = get_rocker_pos(p1, *joints);
            let psi = angle(p3, p1);
            let p4 = (
                p1.0 + dx * (theta - psi).cos() - dy * (theta - psi).sin(),
                p1.1 + dy * (theta - psi).cos() + dx * (theta - psi).sin(),
            );

            joints[0] = p3;
            joints[1] = p1;
            joints[4] = p4;
        }
        window
            .child()
            .unwrap()
            .downcast_ref::<Grid>()
            .unwrap()
            .child_at(0, 0)
            .unwrap()
            .queue_draw();
        glib::ControlFlow::Continue
    });

    // Present window
    window.present();
}
