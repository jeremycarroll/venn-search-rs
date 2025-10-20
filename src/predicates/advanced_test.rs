// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Test predicates for Venn related testing.
//!
//! These predicates are more complex supporting predicates,
//! including invariant validation and saving.

use std::fmt::Debug;
use std::fs::File;
use std::io::{BufWriter, Write};
use crate::context::SearchContext;
use crate::engine::{Predicate, PredicateResult};
use crate::engine::predicate::OpenClose;
use crate::geometry::{Color, NCOLORS, NFACES};
use crate::state::statistics::Counters;
use crate::symmetry::s6::check_solution_canonicality;

const MAX_ITER: usize = 100;
#[derive(Debug)]
pub struct OpenCloseFile {
    old: Option<Box<BufWriter<File>>>,
    prefix: String,
    counter: u32,
}
impl OpenCloseFile {
    pub fn new(prefix:String) -> Self {
        OpenCloseFile {
            old:None,
            prefix: prefix,
            counter: 0,
        }
    }
}
impl OpenClose for OpenCloseFile {
    fn open(&mut self, ctx: &mut SearchContext) -> bool {
        if self.old.is_some() {
            panic!("invariant failure - single use object");
        }
        let filename = format!("{}_{:05}.txt", self.prefix, self.counter);
        let buffered_writer = BufWriter::new(File::create(&filename).expect(format!("Cannot open file: {}", filename).as_str()));
        self.old = ctx.state.output.replace(Box::new(buffered_writer));
        self.counter += 1;
        true
    }

    fn close(&mut self, ctx: &mut SearchContext) {
        let our_boxed_writer = ctx.state.output.take();
        ctx.state.output = self.old.take();
        let mut writer = *(our_boxed_writer.expect("Invariant failure, output field should only be mutated by OpenCloseFile"));
        writer.flush().expect("I/O error on close of file");
    }
}

#[derive(Debug)]
pub struct PrintHeaderPredicate;

impl Predicate for PrintHeaderPredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        let symmetry = check_solution_canonicality(&ctx.state, &ctx.memo);
        let writer = ctx.state.output.as_deref_mut().expect("Must open file to save solution");
        let _ = write!(writer, "## {:?} Solution {} - inner face degrees: {:?}\n\n",symmetry,  ctx.statistics.get(Counters::VennSolutions), ctx.state.current_face_degrees);

        PredicateResult::Success
    }
}

#[derive(Debug)]
pub struct PrintFacesPredicate;

impl Predicate for PrintFacesPredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        let writer = ctx.state.output.as_deref_mut().expect("Must open file to save solution");

        for face_id in 0..NFACES {
            let face = &ctx.state.faces.faces[face_id];
            let face_memo = ctx.memo.faces.get_face(face_id);

            let next_id = face.next_face().map(|id| id as u64).unwrap_or(0);
            let prev_id = face.previous_face().map(|id| id as u64).unwrap_or(0);

            if let Some(cycle_id) = face.current_cycle() {
                let cycle = ctx.memo.cycles.get(cycle_id);
                writeln!(writer,
                    "Face {:2} ({:0width$b}): cycle {:2} = {} [next={:0width$b}, prev={:0width$b}]",
                    face_id,
                    face_memo.colors.bits(),
                    cycle_id,
                    cycle,
                    next_id,
                    prev_id,
                    width = NCOLORS,
                ).unwrap();
            } else {
                writeln!(writer, "Face {:2} ({:0width$b}): UNASSIGNED [next={:0width$b}, prev={:0width$b}]",
                                   face_id, face_memo.colors.bits(), next_id, prev_id,
                                   width = NCOLORS,).unwrap();
            }
        }
        PredicateResult::Success
    }
}

#[derive(Debug)]
pub struct PrintFaceCyclesPredicate;

impl Predicate for PrintFaceCyclesPredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        let writer = ctx.state.output.as_deref_mut().expect("Must open file to save solution");

        writeln!(writer, "\n--- Face Cycles by Color Count ---").unwrap();

        for color_count in 0..=NCOLORS {
            // Find first face with this color count
            let first_face = (0..NFACES).find(|&id| {
                ctx.memo.faces.get_face(id).colors.len() == color_count
            });

            if let Some(start_id) = first_face {
                let _expected_length = ctx.memo.faces.face_degree_by_color_count[color_count] as usize;
                let mut current_id = start_id;
                let mut iterations = 0;

                loop {
                    let _ = write!(writer,"{:0width$b}, ", current_id, width=NCOLORS );
                    iterations += 1;

                    if iterations > MAX_ITER {
                        break;
                    }

                    let next_id = ctx.state.faces.faces[current_id].next_face().expect("Incomplete cycle of faces");
                            if next_id == start_id {
                                break; // Completed cycle
                            }
                    current_id = next_id;
                }
                writeln!(writer, "\ni.e. {} faces", iterations).unwrap();
            }
        }

        PredicateResult::Success
    }
}

#[derive(Debug)]
pub struct PrintEdgeCyclesPredicate {
   validate_length:fn(&usize)->bool,
}

impl PrintEdgeCyclesPredicate {
    pub fn new(validate:Option<fn(&usize)->bool>)->Self {
        Self{validate_length:validate.unwrap_or(|_|true)}
    }
}

impl Predicate for PrintEdgeCyclesPredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        let writer = ctx.state.output.as_deref_mut().expect("Must open file to save solution");

        writeln!(writer, "\n--- Edge Cycles (Curves) by Color ---").unwrap();

        let central_face_id = NFACES - 1; // Central face (all colors)
        let mut total_edges = 0;

        for color_idx in 0..NCOLORS {
            let color = Color::new(color_idx as u8);
            let mut edge_count = 0;

            let mut current_face_id = central_face_id;

            let _ = write!(writer, "Color {}: ", color);


            // Walk the curve
            loop {
                edge_count += 1;
                total_edges += 1;

                let _ = write!(writer, "{:0width$b}, ", current_face_id, width = NCOLORS);

                // Get edge->to for this color at this face
                let link = ctx.state.faces.faces[current_face_id].edge_dynamic[color_idx].get_to().expect("Incomplete cycle of edges");

                current_face_id = link.next.face_id;
                if link.next.color_idx != color_idx {
                    panic!("Curve color changed!");
                }
                if current_face_id == central_face_id || edge_count > MAX_ITER {
                    break;
                }
            }
            let _ = writeln!(writer, " [{} steps]", &edge_count);
            assert!((&self.validate_length)(&edge_count));
        }
        let _ = writeln!(writer, "\n\nGrand total: {} edges", total_edges);

        PredicateResult::Success
    }
}

#[derive(Debug)]
pub struct PrintCornerCountPredicate;

impl Predicate for PrintCornerCountPredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        let writer = ctx.state.output.as_deref_mut().expect("Must open file to save solution");
        writeln!(writer, "\n--- Corner Counts by Color ---").unwrap();

        for color_idx in 0..NCOLORS {
            let color = Color::new(color_idx as u8);
// The corner detection should save state, and then we can just print off what it found.
            let _ = writeln!(writer, "Color {}: unimplemented.", color);
        }
        PredicateResult::Success
    }
}
