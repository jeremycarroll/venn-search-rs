// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Completed color removal optimization.
//!
//! When a color forms a complete closed loop, we can optimize the search by
//! restricting all unassigned faces to cycles omitting that color. This also
//! serves as a disconnection check: if any face needs the completed color,
//! then the curve must be disconnected.
