//! Native equivalents of the installed Animate UI 2.4.2 motion tokens.
//! No timer runs at rest: the view requests frames only while a value is moving.
use std::time::Instant;

pub const TEMPO: f32 = 0.85;

#[derive(Clone, Copy)]
pub struct Spring {
    from: f32,
    velocity: f32,
    target: f32,
    started: Instant,
    stiffness: f32,
    damping: f32,
}

impl Spring {
    pub fn new(value: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            from: value,
            velocity: 0.,
            target: value,
            started: Instant::now(),
            stiffness,
            damping,
        }
    }

    pub fn selection(value: f32) -> Self {
        Self::new(value, 300., 30.)
    }
    pub fn thumb(value: bool) -> Self {
        Self::new(value as u8 as f32, 300., 25.)
    }

    fn sample(&self, now: Instant) -> (f32, f32) {
        let t = now.saturating_duration_since(self.started).as_secs_f32() / TEMPO;
        if t > 1.5 {
            return (self.target, 0.);
        }
        let y = self.from - self.target;
        let v = self.velocity * TEMPO;
        let a = self.damping / 2.;
        let discriminant = a * a - self.stiffness;
        let (position, velocity) = if discriminant.abs() < 0.001 {
            let b = v + a * y;
            let e = (-a * t).exp();
            (e * (y + b * t), e * (b - a * (y + b * t)))
        } else if discriminant < 0. {
            let w = (-discriminant).sqrt();
            let b = (v + a * y) / w;
            let (s, c) = (w * t).sin_cos();
            let e = (-a * t).exp();
            (
                e * (y * c + b * s),
                e * ((b * w - a * y) * c - (y * w + a * b) * s),
            )
        } else {
            let root = discriminant.sqrt();
            let r1 = -a + root;
            let r2 = -a - root;
            let c1 = (v - r2 * y) / (r1 - r2);
            let c2 = y - c1;
            (
                c1 * (r1 * t).exp() + c2 * (r2 * t).exp(),
                c1 * r1 * (r1 * t).exp() + c2 * r2 * (r2 * t).exp(),
            )
        };
        (self.target + position, velocity / TEMPO)
    }

    pub fn value(&self, now: Instant) -> f32 {
        if self.active(now) {
            self.sample(now).0
        } else {
            self.target
        }
    }

    pub fn active(&self, now: Instant) -> bool {
        let (x, v) = self.sample(now);
        (x - self.target).abs() > 0.0005 || v.abs() > 0.005
    }

    pub fn set(&mut self, target: f32, now: Instant, reduced: bool) {
        if reduced {
            self.snap(target, now);
            return;
        }
        if self.target == target {
            return;
        }
        // Preserve both position and velocity on a second click.
        let (from, velocity) = self.sample(now);
        self.from = from;
        self.velocity = velocity;
        self.target = target;
        self.started = now;
    }

    pub fn snap(&mut self, value: f32, now: Instant) {
        self.from = value;
        self.target = value;
        self.velocity = 0.;
        self.started = now;
    }

    pub fn finish(&mut self, now: Instant) {
        self.snap(self.target, now);
    }
}

#[derive(Clone, Copy)]
pub struct Fade {
    from: f32,
    target: f32,
    started: Instant,
    duration: f32,
    curve: [f32; 4],
}

impl Fade {
    pub fn new(value: f32) -> Self {
        Self {
            from: value,
            target: value,
            started: Instant::now(),
            duration: 0.,
            curve: [0.215, 0.61, 0.355, 1.],
        }
    }
    pub fn active(&self, now: Instant) -> bool {
        self.from != self.target
            && now.saturating_duration_since(self.started).as_secs_f32() < self.duration
    }
    pub fn value(&self, now: Instant) -> f32 {
        if !self.active(now) {
            return self.target;
        }
        let t = now.saturating_duration_since(self.started).as_secs_f32() / self.duration;
        self.from + (self.target - self.from) * bezier(t, self.curve)
    }
    pub fn set(&mut self, target: f32, seconds: f32, now: Instant) {
        self.from = self.value(now);
        self.target = target;
        self.started = now;
        self.duration = seconds * TEMPO;
    }
    pub fn finish(&mut self) {
        self.from = self.target;
    }
    pub fn curve(&mut self, curve: [f32; 4]) {
        self.curve = curve;
    }
}

// transition.disclosure: cubic-bezier(0.215, 0.61, 0.355, 1).
fn bezier(x: f32, points: [f32; 4]) -> f32 {
    let curve = |t: f32, a: f32, b: f32| {
        3. * (1. - t).powi(2) * t * a + 3. * (1. - t) * t * t * b + t.powi(3)
    };
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..16 {
        let mid = (lo + hi) * 0.5;
        if curve(mid, points[0], points[2]) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    curve((lo + hi) * 0.5, points[1], points[3])
}

pub struct Panel {
    pub visible: usize,
    target: usize,
    pub opacity: Fade,
    pub offset: Spring,
    leaving: bool,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            visible: 0,
            target: 0,
            opacity: Fade::new(1.),
            offset: Spring::new(0., 300., 35.),
            leaving: false,
        }
    }
    pub fn select(&mut self, target: usize, now: Instant, reduced: bool) {
        self.target = target;
        if self.visible == target {
            self.leaving = false;
            self.opacity.curve([0.25, 0.1, 0.35, 1.]);
            self.opacity.set(1., 0.3, now);
            self.offset.set(0., now, reduced);
        } else {
            self.leaving = true;
            self.opacity.curve([0., 0., 0.58, 1.]);
            self.opacity.set(0., 0.15, now);
            self.offset
                .set(if target > self.visible { -12. } else { 12. }, now, reduced);
        }
    }
    pub fn advance(&mut self, now: Instant, reduced: bool) {
        if self.leaving && !self.opacity.active(now) {
            let direction = if self.target > self.visible { 1. } else { -1. };
            self.visible = self.target;
            self.leaving = false;
            self.opacity.curve([0.25, 0.1, 0.35, 1.]);
            self.opacity.set(1., if reduced { 0.2 } else { 0.3 }, now);
            self.offset
                .snap(if reduced { 0. } else { direction * 12. }, now);
            self.offset.set(0., now, reduced);
        }
    }
    pub fn active(&self, now: Instant) -> bool {
        self.leaving || self.opacity.active(now) || self.offset.active(now)
    }
    pub fn finish_movement(&mut self, now: Instant) {
        self.offset.snap(0., now);
    }
}

pub fn mix(from: u32, to: u32, progress: f32) -> u32 {
    let t = progress.clamp(0., 1.);
    [16, 8, 0].into_iter().fold(0, |color, shift| {
        let a = ((from >> shift) & 255) as f32;
        let b = ((to >> shift) & 255) as f32;
        color | (((a + (b - a) * t).round() as u32) << shift)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn spring_retargets_without_a_position_or_velocity_jump() {
        let now = Instant::now();
        let mut s = Spring::selection(0.);
        s.set(1., now, false);
        let at = now + Duration::from_millis(70);
        let before = s.sample(at);
        s.set(0., at, false);
        let after = s.sample(at);
        assert!((before.0 - after.0).abs() < 0.0001);
        assert!((before.1 - after.1).abs() < 0.0001);
        assert!(!s.active(at + Duration::from_secs(2)));
    }
    #[test]
    fn reduced_motion_snaps_without_requesting_frames() {
        let now = Instant::now();
        let mut s = Spring::thumb(false);
        s.set(1., now, true);
        assert_eq!(s.value(now), 1.);
        assert!(!s.active(now));
    }
    #[test]
    fn disclosure_can_reverse_from_its_current_height() {
        let now = Instant::now();
        let mut f = Fade::new(0.);
        f.set(1., 0.35, now);
        let at = now + Duration::from_millis(85);
        let before = f.value(at);
        assert!(before > 0. && before < 1.);
        f.set(0., 0.35, at);
        assert!((before - f.value(at)).abs() < 0.0001);
        assert_eq!(f.value(at + Duration::from_secs(1)), 0.);
    }
    #[test]
    fn rapid_navigation_shows_the_last_requested_page() {
        let now = Instant::now();
        let mut p = Panel::new();
        p.select(1, now, false);
        p.select(3, now + Duration::from_millis(25), false);
        p.advance(now + Duration::from_millis(200), false);
        assert_eq!(p.visible, 3);
        p.advance(now + Duration::from_secs(2), false);
        assert!(!p.active(now + Duration::from_secs(2)));
    }
}
