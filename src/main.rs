// Copyright (c) 2017 Brandon Thomas <bt@brand.io>, <echelon@gmail.com>

//! ilda-player is a program that plays ILDA laser projection files on laser
//! projector DACs.

extern crate argparse;
extern crate lase;
extern crate point;
extern crate time;

mod letters;

use argparse::ArgumentParser;
use argparse::{Store, StoreTrue};
use lase::Point;
use lase::tools::find_first_etherdream_dac;
use point::PipelinePoint;
use point::SimplePoint;
use std::f32::consts::PI;

pub struct Cycle {
  pub value: i16,
  max: i16,
  min: i16,
  increment: bool,
}

impl Cycle {
  pub fn new(min: i16, max: i16) -> Self {
    Cycle {
      value: 0,
      min,
      max,
      increment: true,
    }
  }
  pub fn count(&mut self, by: i16) {
    let mut newval = self.value;
    if self.increment {
      newval += by;
      if newval > self.max {
        newval = self.max;
        self.increment = false;
      }
    } else {
      newval -= by;
      if newval < self.min {
        newval = self.min;
        self.increment = true;
      }
    }
    self.value = newval;
  }
}

fn new_main() {
  let mut dac = find_first_etherdream_dac().expect("Unable to find DAC");

  let mut i = 0;
  let mut cycle = Cycle::new(-20_000, 20000);

  dac.play_function(move |num_points: u16| {
    let num_points = num_points as usize;
    let mut buf = Vec::new();


    while buf.len() < num_points {
      println!("Counter: {}", cycle.value);
      cycle.count(100);


      let pipeline_point = PipelinePoint::xy_binary(cycle.value as f32, 0.0, true);

      //let pipeline_point = PipelinePoint::xy_rgb(0.0, 0.0, PipelinePoint::MAX_COLOR, PipelinePoint::MAX_COLOR, PipelinePoint::MAX_COLOR);

      let dac_point = pipeline_to_dac(&pipeline_point);
      buf.push(dac_point);
    }

    buf
  }).expect("DAC is broken");
}

fn main() {
  let mut filename = String::new();
  let mut show_blanking = false;
  let mut frame_repeat_number = 50u8;

  { // Limit scope of borrow.
    let mut parser = ArgumentParser::new();

    parser.set_description("ILDA laser projection file player.");
    //parser.refer(&mut filename)
    //    .add_argument("filename", Store, "ILDA file to load");
    parser.refer(&mut show_blanking)
        .add_option(&["-b", "--show-blanking"], StoreTrue,
            "Show the blanking points");
    parser.refer(&mut frame_repeat_number)
        .add_option(&["-r", "--repeat-frames"], Store,
            "Number of times to repeat frames");

    parser.parse_args_or_exit();
  }

  let repeat_frames = frame_repeat_number != 0;


  println!("Searching for EtherDream DAC...");

  let mut dac = find_first_etherdream_dac().expect("Unable to find DAC");

  let mut current_frame = 0;
  let mut current_point = 0;
  let mut frame_repeat_count = 0;

  let mut letter = letters::letter_n();

  let mut letters : Vec<Vec<PipelinePoint>> = Vec::new();

  letters.push(letters::date_2018());
  //letters.push(letters::letter_t());

  let mut current_index = 0;

  dac.play_function(move |num_points: u16| {
    let num_points = num_points as usize;
    let mut buf = Vec::new();

    while buf.len() < num_points {

      /* ///// OLD LETTER CODE /////
      let pipeline_point = letter.get(current_index).unwrap();
      current_index = (current_index + 1) % letter.len();

      let dac_point = pipeline_to_dac(&pipeline_point);
      buf.push(dac_point);*/

      let frame = match letters.get(current_frame) {
        Some(frame) => frame,
        None => {
          // End of animation
          current_frame = 0;
          current_point = 0;
          frame_repeat_count = 0;
          continue;
        }
      };

      let point = match frame.get(current_point) {
        Some(point) => point,
        None => {
          // End of frame
          if repeat_frames && frame_repeat_count < frame_repeat_number {
            current_point = 0;
            frame_repeat_count += 1;
            continue;
          }
          current_frame += 1;
          current_point = 0;
          frame_repeat_count = 0;
          continue;
        },
      };

      current_point += 1;

      /*let simple_point = if point.is_blank && !show_blanking {
        SimplePoint::xy_blank(invert_x(point.x), point.y)
      } else {
        // The DAC supports a wider colorspace than ILDA.
        SimplePoint::xy_rgb(
          invert_x(point.x),
          point.y,
          expand(point.r),
          expand(point.g),
          expand(point.b))
      };*/

      let mut pipeline_point = point.clone();

      let rot = get_rotation(2);

      //x_rotate(&mut pipeline_point, rot);
      y_rotate(&mut pipeline_point, rot);
      //z_rotate(&mut pipeline_point, rot);

      let dac_point = pipeline_to_dac(&pipeline_point);
      buf.push(dac_point);
    }

    buf
  }).expect("Streaming to the DAC is broken.");
}

// TODO: These definitely belong in beam.
fn x_rotate(point: &mut PipelinePoint, theta: f32) {
  let y = point.y * theta.sin();
  point.y = y;
}

fn y_rotate(point: &mut PipelinePoint, theta: f32) {
  let x = point.x * theta.sin();
  point.x = x;
}

fn z_rotate(point: &mut PipelinePoint, theta: f32) {
  let x = point.x * theta.cos() - point.y * theta.sin();
  let y = point.y * theta.cos() + point.x * theta.sin();
  point.x = x;
  point.y = y;
}

/*
TODO: Perspective transform
- https://gamedev.stackexchange.com/questions/44751/2d-camera-perspective-projection-from-3d-coordinates-how
- https://stackoverflow.com/questions/14177744/how-does-perspective-transformation-work-in-pil
*/

// TODO: Time period functions would be good for beam.
fn get_rotation(second_duration: i32) -> f32 {
  // NB: second_duration must be in [1, 60].
  const TWO_PI: f32 = PI * 2.0f32;

  let now = time::now();

  let tm_part_sec = now.tm_sec % second_duration; // NB: Defines the period.

  let second_fraction = now.tm_nsec as f32 / 1_000_000_000.0;

  let pt = (tm_part_sec - 1) as f32 + second_fraction; // NB: Minus one second!

  let sec_fraction = pt / second_duration as f32;

  sec_fraction * TWO_PI
}

fn get_simple_rotation() -> f32 {
  const TWO_PI: f32 = PI * 2.0f32;
  let now = time::now();
  let second_fraction = now.tm_nsec as f32 / 1_000_000_000.0;
  //let minute_fraction = now.tm_sec as f32 / 60.0;
  second_fraction * TWO_PI
}

fn invert_x(x_coordinate: i16) -> i16 {
  // Compensate for flipped x-coordinate plane.
  // TODO: This might be a bug in the ILDA parser, or perhaps Etherdream.rs.
  x_coordinate.saturating_mul(-1)
}

#[inline]
fn expand(color: u8) -> u16 {
  (color as u16) * 257 // or the incorrect: (color as u16) << 8
}

// TODO: Move this functionality into lase.rs, etherdream.rs, and point.rs.
fn simple_to_dac(point: &SimplePoint) -> Point {
  if point.is_blank {
    Point::xy_blank(point.x, point.y)
  } else {
    let r = expand(point.r);
    let g = expand(point.g);
    let b = expand(point.b);
    Point::xy_rgb(point.x, point.y, r, g, b)
  }
}

// TODO: Move this functionality into lase.rs, etherdream.rs, and point.rs.
fn pipeline_to_dac(point: &PipelinePoint) -> Point {
  let point = point.into_simple_pt();
  simple_to_dac(&point)
}
