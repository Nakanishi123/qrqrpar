//! Render a QR code into svg string.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DirectedSegment {
    sx: i16,
    sy: i16,
    ex: i16,
    ey: i16,
}

impl DirectedSegment {
    fn new(sx: i16, sy: i16, ex: i16, ey: i16) -> DirectedSegment {
        Self { sx, sy, ex, ey }
    }

    fn from_coord(x: i16, y: i16) -> [DirectedSegment; 4] {
        [
            Self::new(x, y, x + 1, y),
            Self::new(x + 1, y, x + 1, y + 1),
            Self::new(x + 1, y + 1, x, y + 1),
            Self::new(x, y + 1, x, y),
        ]
    }

    /// Returns the reversed segment
    fn reversed(&self) -> DirectedSegment {
        Self::new(self.ex, self.ey, self.sx, self.sy)
    }

    /// Determines the direction of the segment
    fn direction(&self) -> Direction {
        match (self.sx == self.ex, self.sy < self.ey, self.sx < self.ex) {
            (true, true, _) => Direction::Down,
            (true, false, _) => Direction::Up,
            (false, _, true) => Direction::Right,
            (false, _, false) => Direction::Left,
        }
    }

    fn direction_to(&self, direction: Direction) -> DirectedSegment {
        match direction {
            Direction::Right => Self::new(self.ex, self.ey, self.ex + 1, self.ey),
            Direction::Down => Self::new(self.ex, self.ey, self.ex, self.ey + 1),
            Direction::Left => Self::new(self.ex, self.ey, self.ex - 1, self.ey),
            Direction::Up => Self::new(self.ex, self.ey, self.ex, self.ey - 1),
        }
    }

    fn start_coord(&self) -> [i16; 2] {
        [self.sx, self.sy]
    }

    fn end_coord(&self) -> [i16; 2] {
        [self.ex, self.ey]
    }
}

#[derive(Debug, Clone)]
pub struct DirectedSegments {
    segments: hashbrown::HashSet<DirectedSegment>,
}

impl DirectedSegments {
    pub fn new() -> Self {
        Self {
            segments: hashbrown::HashSet::new(),
        }
    }

    /// if the opposite segment is exists, remove it, otherwise add it
    fn add_or_remove_segment(&mut self, segment: DirectedSegment) {
        if !self.segments.remove(&segment.reversed()) {
            self.segments.insert(segment);
        }
    }

    pub fn add_or_remove(&mut self, x: i16, y: i16) {
        for segment in DirectedSegment::from_coord(x, y).iter() {
            self.add_or_remove_segment(*segment);
        }
    }

    fn pop(&mut self) -> Option<DirectedSegment> {
        if let Some(segment) = self.segments.iter().next().copied() {
            self.segments.remove(&segment);
            return Some(segment);
        }
        None
    }

    fn pop_segment(&mut self, segment: DirectedSegment) -> Option<DirectedSegment> {
        if self.segments.remove(&segment) {
            return Some(segment);
        }
        None
    }

    /// Returns the next segment and removes it from hashset
    fn pop_next(&mut self, segment: DirectedSegment) -> Option<DirectedSegment> {
        for alternative in &Self::alternative_segments(segment) {
            if self.segments.contains(alternative) {
                return self.pop_segment(*alternative);
            }
        }
        None
    }

    fn alternative_segments(segment: DirectedSegment) -> [DirectedSegment; 3] {
        match segment.direction() {
            Direction::Right => [
                segment.direction_to(Direction::Down),
                segment.direction_to(Direction::Up),
                segment.direction_to(Direction::Right),
            ],
            Direction::Down => [
                segment.direction_to(Direction::Left),
                segment.direction_to(Direction::Right),
                segment.direction_to(Direction::Down),
            ],
            Direction::Left => [
                segment.direction_to(Direction::Up),
                segment.direction_to(Direction::Down),
                segment.direction_to(Direction::Left),
            ],
            Direction::Up => [
                segment.direction_to(Direction::Right),
                segment.direction_to(Direction::Left),
                segment.direction_to(Direction::Up),
            ],
        }
    }

    /// Returns a list of directed line segments whose endpoints are corners
    /// and removes the line segments related to them from hashset
    fn pop_corners(&mut self) -> Option<Vec<DirectedSegment>> {
        if let Some(start_segment) = self.pop() {
            let mut corners = vec![];
            let mut current_segment = start_segment;
            while let Some(next_segment) = self.pop_next(current_segment) {
                if current_segment.direction() != next_segment.direction() {
                    corners.push(current_segment);
                }
                current_segment = next_segment;
                if current_segment.end_coord() == start_segment.start_coord() {
                    break;
                }
            }
            if current_segment.direction() != start_segment.direction() {
                corners.push(current_segment);
            }
            return Some(corners);
        }
        None
    }

    /// Returns a list of directed line segments whose endpoints are corners
    /// and removes the line segments related to them from hashset
    fn pop_corners_list(&mut self) -> Vec<Vec<DirectedSegment>> {
        let mut corners_list = vec![];
        while let Some(corners) = self.pop_corners() {
            corners_list.push(corners);
        }
        corners_list
    }

    /// Convert to path string.
    /// Breaking change
    pub fn to_path_square_mut(&mut self) -> String {
        let mut s = String::new();
        let corners_list = self.pop_corners_list();
        for corners in corners_list.iter() {
            s.push_str(&format!("M{} {}", corners[0].ex, corners[0].ey));
            for seg in corners.windows(2) {
                if let [before, current] = seg {
                    let offset_x = current.ex - before.ex;
                    let offset_y = current.ey - before.ey;
                    match offset_x {
                        0 => s.push_str(&format!("v{}", offset_y)),
                        _ => s.push_str(&format!("h{}", offset_x)),
                    }
                }
            }
            s.push('Z');
        }
        s
    }

    /// Convert to path string.
    /// Breaking change
    pub fn to_path_round_mut(&mut self) -> String {
        let mut s = String::new();
        let corners_list = self.pop_corners_list();
        for corners in corners_list.iter() {
            let start_segment = corners[0];
            let [start_x, start_y] = start_segment.end_coord();
            match start_segment.direction() {
                Direction::Right => s.push_str(&format!("M{}.5 {}", start_x - 1, start_y)),
                Direction::Down => s.push_str(&format!("M{} {}.5", start_x, start_y - 1)),
                Direction::Left => s.push_str(&format!("M{}.5 {}", start_x, start_y)),
                Direction::Up => s.push_str(&format!("M{} {}.5", start_x, start_y)),
            }

            let mut before_segment = corners[0];
            for current_segment in corners.iter().skip(1).chain(corners.iter().take(1)) {
                let dx = match (before_segment.direction(), current_segment.direction()) {
                    (Direction::Left, _) | (_, Direction::Left) => "-.5",
                    (Direction::Right, _) | (_, Direction::Right) => " .5",
                    _ => unreachable!(),
                };
                let dy = match (before_segment.direction(), current_segment.direction()) {
                    (Direction::Up, _) | (_, Direction::Up) => "-.5",
                    (Direction::Down, _) | (_, Direction::Down) => " .5",
                    _ => unreachable!(),
                };
                let (dx1, dy1) = match current_segment.direction() {
                    Direction::Up | Direction::Down => (dx, " 0"),
                    _ => ("0 ", dy),
                };
                s.push_str(&format!("q{dx1}{dy1}{dx}{dy}"));

                let offset_x = current_segment.ex - before_segment.ex;
                let offset_y = current_segment.ey - before_segment.ey;
                if offset_y.abs() > 1 {
                    s.push_str(&format!("v{}", offset_y - offset_y / offset_y.abs()));
                } else if offset_x.abs() > 1 {
                    s.push_str(&format!("h{}", offset_x - offset_x / offset_x.abs()));
                }
                before_segment = *current_segment;
            }
            s.push('Z');
        }
        s
    }
}
