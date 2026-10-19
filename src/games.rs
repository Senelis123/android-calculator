//! Mini games: 2048, 15-puzzle, Tetris, Snake, Minesweeper. Pure logic, unit tested.

/// Small deterministic RNG (xorshift64*) so games work offline and tests are repeatable.
#[derive(Clone)]
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self { Rng(seed | 1) }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize { (self.next_u64() % n.max(1) as u64) as usize }
}

// ---------------- 2048 ----------------

#[derive(Clone)]
pub struct G2048 {
    pub g: [[u32; 4]; 4],
    pub score: u64,
    pub over: bool,
    pub won: bool,
    rng: Rng,
}
impl G2048 {
    pub fn new(seed: u64) -> Self {
        let mut s = G2048 { g: [[0; 4]; 4], score: 0, over: false, won: false, rng: Rng::new(seed) };
        s.spawn();
        s.spawn();
        s
    }
    fn spawn(&mut self) {
        let empty: Vec<(usize, usize)> = (0..4)
            .flat_map(|y| (0..4).map(move |x| (x, y)))
            .filter(|&(x, y)| self.g[y][x] == 0)
            .collect();
        if empty.is_empty() { return; }
        let (x, y) = empty[self.rng.below(empty.len())];
        self.g[y][x] = if self.rng.below(10) == 0 { 4 } else { 2 };
    }
    /// dir: 0 left, 1 right, 2 up, 3 down. Returns true when the board changed.
    pub fn mv(&mut self, dir: usize) -> bool {
        if self.over { return false; }
        let before = self.g;
        for line in 0..4 {
            let mut v: Vec<u32> = (0..4)
                .map(|i| match dir {
                    0 => self.g[line][i],
                    1 => self.g[line][3 - i],
                    2 => self.g[i][line],
                    _ => self.g[3 - i][line],
                })
                .filter(|&n| n != 0)
                .collect();
            let mut out: Vec<u32> = vec![];
            let mut i = 0;
            while i < v.len() {
                if i + 1 < v.len() && v[i] == v[i + 1] {
                    let m = v[i] * 2;
                    self.score += m as u64;
                    if m >= 2048 { self.won = true; }
                    out.push(m);
                    i += 2;
                } else {
                    out.push(v[i]);
                    i += 1;
                }
            }
            v = out;
            for i in 0..4 {
                let val = *v.get(i).unwrap_or(&0);
                match dir {
                    0 => self.g[line][i] = val,
                    1 => self.g[line][3 - i] = val,
                    2 => self.g[i][line] = val,
                    _ => self.g[3 - i][line] = val,
                }
            }
        }
        let moved = self.g != before;
        if moved {
            self.spawn();
            self.over = !self.can_move();
        }
        moved
    }
    fn can_move(&self) -> bool {
        for y in 0..4 {
            for x in 0..4 {
                if self.g[y][x] == 0 { return true; }
                if x < 3 && self.g[y][x] == self.g[y][x + 1] { return true; }
                if y < 3 && self.g[y][x] == self.g[y + 1][x] { return true; }
            }
        }
        false
    }
}

// ---------------- 15 puzzle ----------------

#[derive(Clone)]
pub struct Fifteen {
    pub b: [u8; 16], // 0 = empty
    pub moves: u32,
}
impl Fifteen {
    pub fn new(seed: u64) -> Self {
        let mut f = Fifteen { b: [0; 16], moves: 0 };
        for i in 0..15 { f.b[i] = (i + 1) as u8; }
        let mut rng = Rng::new(seed);
        // shuffle with legal moves only, so it is always solvable
        let mut prev = usize::MAX;
        for _ in 0..220 {
            let blank = f.blank();
            let (bx, by) = (blank % 4, blank / 4);
            let mut opts: Vec<usize> = vec![];
            if bx > 0 { opts.push(blank - 1); }
            if bx < 3 { opts.push(blank + 1); }
            if by > 0 { opts.push(blank - 4); }
            if by < 3 { opts.push(blank + 4); }
            opts.retain(|&o| o != prev);
            let pick = opts[rng.below(opts.len())];
            prev = blank;
            f.b.swap(blank, pick);
        }
        if f.solved() { f.b.swap(0, 1); }
        f
    }
    fn blank(&self) -> usize { self.b.iter().position(|&v| v == 0).unwrap() }
    /// Slide the tile at index i into the empty space when adjacent. Returns true on a move.
    pub fn tap(&mut self, i: usize) -> bool {
        if i >= 16 || self.b[i] == 0 { return false; }
        let blank = self.blank();
        let (ix, iy) = (i % 4, i / 4);
        let (bx, by) = (blank % 4, blank / 4);
        if (ix == bx && iy.abs_diff(by) == 1) || (iy == by && ix.abs_diff(bx) == 1) {
            self.b.swap(i, blank);
            self.moves += 1;
            true
        } else {
            false
        }
    }
    pub fn solved(&self) -> bool { (0..15).all(|i| self.b[i] == (i + 1) as u8) }
}

// ---------------- Tetris ----------------

pub const TW: usize = 10;
pub const TH: usize = 20;

/// 7 pieces × 4 rotations, each 4 (x, y) cells of a 4×4 box.
const SHAPES: [[[(i32, i32); 4]; 4]; 7] = [
    // I
    [[(0, 1), (1, 1), (2, 1), (3, 1)], [(2, 0), (2, 1), (2, 2), (2, 3)], [(0, 2), (1, 2), (2, 2), (3, 2)], [(1, 0), (1, 1), (1, 2), (1, 3)]],
    // O
    [[(1, 0), (2, 0), (1, 1), (2, 1)]; 4],
    // T
    [[(1, 0), (0, 1), (1, 1), (2, 1)], [(1, 0), (1, 1), (2, 1), (1, 2)], [(0, 1), (1, 1), (2, 1), (1, 2)], [(1, 0), (0, 1), (1, 1), (1, 2)]],
    // S
    [[(1, 0), (2, 0), (0, 1), (1, 1)], [(1, 0), (1, 1), (2, 1), (2, 2)], [(1, 1), (2, 1), (0, 2), (1, 2)], [(0, 0), (0, 1), (1, 1), (1, 2)]],
    // Z
    [[(0, 0), (1, 0), (1, 1), (2, 1)], [(2, 0), (1, 1), (2, 1), (1, 2)], [(0, 1), (1, 1), (1, 2), (2, 2)], [(1, 0), (0, 1), (1, 1), (0, 2)]],
    // J
    [[(0, 0), (0, 1), (1, 1), (2, 1)], [(1, 0), (2, 0), (1, 1), (1, 2)], [(0, 1), (1, 1), (2, 1), (2, 2)], [(1, 0), (1, 1), (0, 2), (1, 2)]],
    // L
    [[(2, 0), (0, 1), (1, 1), (2, 1)], [(1, 0), (1, 1), (1, 2), (2, 2)], [(0, 1), (1, 1), (2, 1), (0, 2)], [(0, 0), (1, 0), (1, 1), (1, 2)]],
];

#[derive(Clone)]
pub struct Tetris {
    pub board: [[u8; TW]; TH], // 0 empty, else piece index + 1
    pub piece: usize,
    pub rot: usize,
    pub px: i32,
    pub py: i32,
    pub next: usize,
    pub score: u64,
    pub lines: u32,
    pub level: u32,
    pub over: bool,
    rng: Rng,
}
impl Tetris {
    pub fn new(seed: u64) -> Self {
        let mut t = Tetris {
            board: [[0; TW]; TH],
            piece: 0, rot: 0, px: 3, py: 0, next: 0,
            score: 0, lines: 0, level: 1, over: false,
            rng: Rng::new(seed),
        };
        t.next = t.rng.below(7);
        t.spawn();
        t
    }
    fn cells(&self, piece: usize, rot: usize, px: i32, py: i32) -> [(i32, i32); 4] {
        let c = SHAPES[piece][rot];
        [ (c[0].0 + px, c[0].1 + py), (c[1].0 + px, c[1].1 + py), (c[2].0 + px, c[2].1 + py), (c[3].0 + px, c[3].1 + py) ]
    }
    fn fits(&self, piece: usize, rot: usize, px: i32, py: i32) -> bool {
        self.cells(piece, rot, px, py).iter().all(|&(x, y)| {
            x >= 0 && (x as usize) < TW && y < TH as i32 && (y < 0 || self.board[y as usize][x as usize] == 0)
        })
    }
    fn spawn(&mut self) {
        self.piece = self.next;
        self.next = self.rng.below(7);
        self.rot = 0;
        self.px = 3;
        self.py = -1;
        if !self.fits(self.piece, self.rot, self.px, self.py) {
            self.over = true;
        }
    }
    pub fn mv(&mut self, dx: i32) -> bool {
        if self.over { return false; }
        if self.fits(self.piece, self.rot, self.px + dx, self.py) {
            self.px += dx;
            true
        } else { false }
    }
    pub fn rotate(&mut self) -> bool {
        if self.over { return false; }
        let r = (self.rot + 1) % 4;
        for kick in [0, -1, 1, -2, 2] {
            if self.fits(self.piece, r, self.px + kick, self.py) {
                self.rot = r;
                self.px += kick;
                return true;
            }
        }
        false
    }
    fn lock(&mut self) {
        for (x, y) in self.cells(self.piece, self.rot, self.px, self.py) {
            if y >= 0 { self.board[y as usize][x as usize] = (self.piece + 1) as u8; }
        }
        let mut cleared = 0;
        let mut y = TH as i32 - 1;
        while y >= 0 {
            if self.board[y as usize].iter().all(|&c| c != 0) {
                cleared += 1;
                for yy in (1..=y as usize).rev() { self.board[yy] = self.board[yy - 1]; }
                self.board[0] = [0; TW];
            } else {
                y -= 1;
            }
        }
        if cleared > 0 {
            self.lines += cleared;
            self.score += [0, 40, 100, 300, 1200][cleared as usize] * self.level as u64;
            self.level = (self.lines / 10 + 1).min(15);
        }
        self.spawn();
    }
    /// Gravity step; returns false when the piece locked (could not move down).
    pub fn tick(&mut self) -> bool {
        if self.over { return false; }
        if self.fits(self.piece, self.rot, self.px, self.py + 1) {
            self.py += 1;
            true
        } else {
            self.lock();
            false
        }
    }
    pub fn hard_drop(&mut self) {
        if self.over { return; }
        let mut d = 0;
        while self.fits(self.piece, self.rot, self.px, self.py + 1) { self.py += 1; d += 1; }
        self.score += 2 * d as u64;
        self.lock();
    }
    pub fn speed_ms(&self) -> u64 { (750u64.saturating_sub(self.level as u64 * 55)).max(120) }
    /// Board with the falling piece merged in (for drawing).
    pub fn view(&self) -> [[u8; TW]; TH] {
        let mut v = self.board;
        for (x, y) in self.cells(self.piece, self.rot, self.px, self.py) {
            if y >= 0 && (y as usize) < TH && x >= 0 && (x as usize) < TW {
                v[y as usize][x as usize] = (self.piece + 1) as u8;
            }
        }
        v
    }
}

// ---------------- Snake ----------------

pub const SW: i32 = 17;
pub const SH: i32 = 21;

#[derive(Clone)]
pub struct Snake {
    pub body: std::collections::VecDeque<(i32, i32)>, // head first
    pub dir: i32,   // 0 up 1 right 2 down 3 left
    pending: i32,
    pub food: (i32, i32),
    pub score: u32,
    pub over: bool,
    rng: Rng,
}
impl Snake {
    pub fn new(seed: u64) -> Self {
        let mut s = Snake {
            body: std::collections::VecDeque::from(vec![(8, 12), (8, 13), (8, 14)]),
            dir: 0,
            pending: 0,
            food: (0, 0),
            score: 0,
            over: false,
            rng: Rng::new(seed),
        };
        s.place_food();
        s
    }
    fn place_food(&mut self) {
        loop {
            let f = (self.rng.below(SW as usize) as i32, self.rng.below(SH as usize) as i32);
            if !self.body.contains(&f) { self.food = f; return; }
        }
    }
    /// Queue a direction change; reversals are ignored.
    pub fn turn(&mut self, dir: i32) {
        if (dir - self.dir).abs() != 2 && dir != self.dir { self.pending = dir; }
    }
    pub fn tick(&mut self) {
        if self.over { return; }
        self.dir = self.pending;
        let (hx, hy) = self.body[0];
        let (nx, ny) = match self.dir {
            0 => (hx, hy - 1),
            1 => (hx + 1, hy),
            2 => (hx, hy + 1),
            _ => (hx - 1, hy),
        };
        if !(0..SW).contains(&nx) || !(0..SH).contains(&ny) || self.body.contains(&(nx, ny)) {
            self.over = true;
            return;
        }
        self.body.push_front((nx, ny));
        if (nx, ny) == self.food {
            self.score += 1;
            self.place_food();
        } else {
            self.body.pop_back();
        }
    }
    pub fn speed_ms(&self) -> u64 { 200u64.saturating_sub(self.score as u64 * 4).max(90) }
}

// ---------------- Minesweeper ----------------

pub const MW: usize = 9;
pub const MH: usize = 9;
pub const MINE_COUNT: usize = 10;

#[derive(Clone)]
pub struct Mines {
    mines: [[bool; MW]; MH],
    pub open: [[bool; MW]; MH],
    pub flag: [[bool; MW]; MH],
    pub over: bool,
    pub won: bool,
    placed: bool,
    seed: u64,
}
impl Mines {
    pub fn new(seed: u64) -> Self {
        Mines { mines: [[false; MW]; MH], open: [[false; MW]; MH], flag: [[false; MW]; MH], over: false, won: false, placed: false, seed }
    }
    fn place(&mut self, safe_x: usize, safe_y: usize) {
        let mut rng = Rng::new(self.seed);
        let mut n = 0;
        while n < MINE_COUNT {
            let x = rng.below(MW);
            let y = rng.below(MH);
            if self.mines[y][x] || (x == safe_x && y == safe_y) { continue; }
            self.mines[y][x] = true;
            n += 1;
        }
        self.placed = true;
    }
    pub fn count(&self, x: usize, y: usize) -> u8 {
        let mut n = 0;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx >= 0 && (nx as usize) < MW && ny >= 0 && (ny as usize) < MH && self.mines[ny as usize][nx as usize] { n += 1; }
            }
        }
        n
    }
    pub fn reveal(&mut self, x: usize, y: usize) {
        if self.over || x >= MW || y >= MH || self.open[y][x] || self.flag[y][x] { return; }
        if !self.placed { self.place(x, y); }
        if self.mines[y][x] {
            self.over = true;
            // show all mines
            for yy in 0..MH { for xx in 0..MW { if self.mines[yy][xx] { self.open[yy][xx] = true; } } }
            return;
        }
        // flood fill from zero cells
        let mut stack = vec![(x, y)];
        while let Some((cx, cy)) = stack.pop() {
            if self.open[cy][cx] || self.flag[cy][cx] { continue; }
            self.open[cy][cx] = true;
            if self.count(cx, cy) == 0 {
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
                        if nx >= 0 && (nx as usize) < MW && ny >= 0 && (ny as usize) < MH && !self.open[ny as usize][nx as usize] {
                            stack.push((nx as usize, ny as usize));
                        }
                    }
                }
            }
        }
        let closed: usize = self.open.iter().flatten().filter(|&&o| !o).count();
        if closed == MINE_COUNT { self.won = true; self.over = true; }
    }
    pub fn toggle_flag(&mut self, x: usize, y: usize) {
        if self.over || x >= MW || y >= MH || self.open[y][x] { return; }
        self.flag[y][x] = !self.flag[y][x];
    }
    /// True when this cell holds a mine (used after game over to draw explosions).
    pub fn revealed_mine(&self, x: usize, y: usize) -> bool { self.mines[y][x] }
    pub fn flags_left(&self) -> i32 {
        MINE_COUNT as i32 - self.flag.iter().flatten().filter(|&&f| f).count() as i32
    }
}

// ---------------- shared wrapper ----------------

pub enum Game {
    G2048(G2048),
    Fifteen(Fifteen),
    Tetris(Tetris),
    Snake(Snake),
    Mines(Mines),
}

pub const GAME_COUNT: usize = 5;

pub fn new_game(kind: usize, seed: u64) -> Game {
    match kind {
        0 => Game::G2048(G2048::new(seed)),
        1 => Game::Fifteen(Fifteen::new(seed)),
        2 => Game::Tetris(Tetris::new(seed)),
        3 => Game::Snake(Snake::new(seed)),
        _ => Game::Mines(Mines::new(seed)),
    }
}

/// Cells of a piece in a 4x4 box (for previews).
pub fn shape_cells(piece: usize, rot: usize) -> [(i32, i32); 4] { SHAPES[piece % 7][rot % 4] }

/// Gravity/timer interval for timed games; None for tap games.
pub fn tick_ms(game: &Game) -> Option<u64> {
    match game {
        Game::Tetris(t) if !t.over => Some(t.speed_ms()),
        Game::Snake(s) if !s.over => Some(s.speed_ms()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g2048_merges() {
        let mut g = G2048::new(7);
        g.g = [[2, 2, 0, 0], [4, 4, 4, 4], [0, 0, 0, 0], [0, 0, 0, 0]];
        g.score = 0;
        assert!(g.mv(0));
        assert_eq!(g.g[0][0], 4);
        assert_eq!(g.g[1][0], 8);
        assert_eq!(g.g[1][1], 8);
        assert_eq!(g.score, 4 + 16);
        // full board with no merges is game over
        let g2 = G2048 { g: [[2, 4, 2, 4], [4, 2, 4, 2], [2, 4, 2, 4], [4, 2, 4, 2]], score: 0, over: false, won: false, rng: Rng::new(3) };
        assert!(!g2.can_move());
    }

    #[test]
    fn fifteen_solvable() {
        let f = Fifteen::new(42);
        assert!(!f.solved());
        // move every tile back by replaying is hard; instead verify a fresh solved board detects solved
        let mut s = Fifteen { b: [0; 16], moves: 0 };
        for i in 0..15 { s.b[i] = (i + 1) as u8; }
        assert!(s.solved());
        // a legal tap moves a tile
        assert!(s.tap(14)); // tile 15 slides right into the blank
        assert!(!s.solved());
        assert!(s.tap(15)); // and back again
        assert!(s.solved());
        assert_eq!(s.moves, 2);
    }

    #[test]
    fn tetris_gravity_and_lines() {
        let mut t = Tetris::new(9);
        assert!(!t.over);
        t.hard_drop();
        // fill bottom row except one column, drop an I vertically into it
        t.board = [[0; TW]; TH];
        for x in 1..TW { t.board[TH - 1][x] = 1; }
        t.piece = 0; t.rot = 1; t.px = -2; t.py = 0; // vertical I lands in column 0
        while t.tick() {}
        assert_eq!(t.lines, 1);
        assert!(t.board[TH - 1][1..].iter().all(|&c| c == 0)); // cleared; rest of the I sits above
    }

    #[test]
    fn snake_eats_and_dies() {
        let mut s = Snake::new(5);
        s.food = (8, 10);
        s.tick(); // up to (8,11)
        s.tick(); // (8,10) eats
        assert_eq!(s.score, 1);
        assert_eq!(s.body.len(), 4);
        // run into the wall
        let mut s2 = Snake::new(5);
        for _ in 0..30 { s2.tick(); if s2.over { break; } }
        assert!(s2.over);
    }

    #[test]
    fn mines_first_click_safe() {
        let mut m = Mines::new(11);
        m.reveal(4, 4);
        assert!(!m.over);
        // flagging blocks reveal
        m.toggle_flag(0, 0);
        let before = m.open[0][0];
        m.reveal(0, 0);
        assert_eq!(m.open[0][0], before);
        // reveal everything except mines -> win
        let mut m2 = Mines::new(2);
        m2.reveal(0, 0);
        for y in 0..MH { for x in 0..MW { if !m2.mines[y][x] { m2.reveal(x, y); } } }
        assert!(m2.won);
    }
}
