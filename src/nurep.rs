extern crate native;
extern crate sdl2;
extern crate serialize;

use serialize::json;
use std::cmp;
use std::io;
use std::os;

mod drawing;
mod state;

fn print_usage() {
    println!("Usage: nurep <data_path>");
}

fn main() {
    let mut quit = false;
    let args: Vec<String> = os::args();
    if args.len() != 2 {
        print_usage();
        return;
    }
    let data_path = args[1].as_slice();

    let json = io::File::open(&Path::new(data_path)).read_to_string().unwrap();
    let game: state::Game = json::decode(json.as_slice()).unwrap();

    sdl2::init(sdl2::INIT_VIDEO);

    let window = match sdl2::video::Window::new(
            "nurep",
            sdl2::video::PosCentered,
            sdl2::video::PosCentered,
            0,
            0,
            sdl2::video::FULLSCREEN_DESKTOP) {
        Ok(window) => window,
        Err(err) => panic!(format!("failed to create window: {}", err))
    };

    let renderer = match sdl2::render::Renderer::from_window(
            window,
            sdl2::render::DriverAuto,
            sdl2::render::ACCELERATED) {
        Ok(renderer) => renderer,
        Err(err) => panic!(format!("failed to create renderer: {}", err))
    };

    // TODO: Somehow know the right display index instead of just picking 0
    let (screen_width, screen_height) = match sdl2::video::get_current_display_mode(0) {
        Ok(display_mode) => (display_mode.w as i32, display_mode.h as i32),
        Err(err) => panic!(format!("failed to retrieve display size: {}", err))
    };
    let draw_size = cmp::min(screen_width, screen_height);
    let scale_factor = 0.95 * (draw_size as f64) / (cmp::max(game.cluster.dimensions.val0(), game.cluster.dimensions.val1()) as f64);


    let mut state = State {
        turn: 1,
        game: game,
        draw_size: draw_size,
        draw_offsets: ((screen_width - draw_size) / 2, ((screen_height - draw_size) / 2) + (draw_size / 40)),
        scale_factor: scale_factor,
    };

    'main: loop {
        if state.turn > state.game.num_turns {
            break;
        }

        let start_time = sdl2::timer::get_ticks();

        for &a in actions().iter() {
            match a {
                Quit => {
                    quit = true;
                    break 'main;
                },
            };
        }

        let _ = draw(&renderer, &state);
        renderer.present();

        state.turn += 1;
        let elapsed = sdl2::timer::get_ticks() - start_time;
        if elapsed < 500 {
            sdl2::timer::delay(500 - elapsed);
        }
    }

    if !quit {
        'quit: loop {
            for &a in actions().iter() {
                match a {
                    _ => break 'quit,
                };
            }
        }
    }

    sdl2::quit();
}

struct State {
    pub turn: i32,
    pub game: state::Game,
    pub draw_size: i32,
    pub draw_offsets: (i32, i32),
    pub scale_factor: f64,
}

enum Action {
    Quit,
}

fn actions() -> Vec<Action> {
    let mut actions: Vec<Action> = Vec::new();

    loop {
        match sdl2::event::poll_event() {
            sdl2::event::QuitEvent(_) => actions.push(Quit),
            sdl2::event::KeyDownEvent(_, _, key, _, _) => {
                if key == sdl2::keycode::EscapeKey {
                    actions.push(Quit);
                }
            },
            sdl2::event::NoEvent => break,
            _ => {}
        }
    }

    actions
}

fn pick_color(owner_id: i32) -> sdl2::pixels::Color {
    match owner_id {
        0  => sdl2::pixels::RGB(0x80, 0x80, 0x80),
        1  => sdl2::pixels::RGB(0x00, 0xff, 0x00),
        2  => sdl2::pixels::RGB(0x00, 0xff, 0x80),
        3  => sdl2::pixels::RGB(0x00, 0xff, 0xff),
        4  => sdl2::pixels::RGB(0x00, 0x80, 0xff),
        5  => sdl2::pixels::RGB(0x00, 0x00, 0xff),
        6  => sdl2::pixels::RGB(0x80, 0x00, 0xff),
        7  => sdl2::pixels::RGB(0xff, 0x00, 0xc0),
        8  => sdl2::pixels::RGB(0xff, 0x00, 0x00),
        9  => sdl2::pixels::RGB(0xff, 0x80, 0x00),
        10 => sdl2::pixels::RGB(0xff, 0xff, 0x00),
        11 => sdl2::pixels::RGB(0x80, 0xff, 0x00),
        _  => sdl2::pixels::RGB(0xff, 0xff, 0xff),
    }
}

/// Transforms a coordinate pair from game coordinates to screen coordinates.
fn transform_coord(state: &State, coord: (i32, i32)) -> (i32, i32) {
    let (x, y) = coord;
    let (scaled_x, scaled_y) = (((x as f64) * state.scale_factor) as i32, ((y as f64 * state.scale_factor)) as i32);
    let (offset_x, offset_y) = state.draw_offsets;
    (scaled_x + offset_x, scaled_y + offset_y)
}

#[must_use]
fn draw(renderer: &sdl2::render::Renderer, state: &State) -> sdl2::SdlResult<()> {
    // Draw background
    let radius = state.draw_size / 175;
    try!(renderer.set_draw_color(sdl2::pixels::RGB(0, 0, 0)));
    try!(renderer.clear());

    // Draw connections
    for conn in state.game.cluster.connections.iter() {
        let position_a = transform_coord(state, match state.game.cluster.planets.iter().find(|x| x.id == conn.id_a) {
            Some(x) => x.position,
            None => continue,
        });
        let position_b = transform_coord(state, match state.game.cluster.planets.iter().find(|x| x.id == conn.id_b) {
            Some(x) => x.position,
            None => continue,
        });
        let color = pick_color(0);
        try!(drawing::draw_line(renderer, position_a, position_b, color));
    }

    // Draw planets
    for planet in state.game.cluster.planets.iter() {
        let owner = match state.game.planet_to_owners.get(&planet.id) {
            Some(turn_to_owner) => match turn_to_owner.get(&state.turn) {
                Some(x) => *x,
                None => -1,
            },
            None => -1,
        };
        let color = pick_color(owner);
        let (x, y) = transform_coord(state, planet.position);
        try!(drawing::draw_circle(renderer, (x, y), radius, color));
    }

    Ok(())
}
