// since vector math doesn't work in const, use macros instead

macro_rules! vec2 {
    ($vec2:ident * $scalar:expr) => {{
        Vec2::new($vec2.x * $scalar, $vec2.y * $scalar)
    }};
    ($vec2:ident / $scalar:expr) => {{
        Vec2::new($vec2.x / $scalar, $vec2.y / $scalar)
    }};
    ($vec2:ident + ($scalar0:expr, $scalar1:expr)) => {{
        Vec2::new($vec2.x + $scalar0, $vec2.y + $scalar1)
    }};
    ($vec2:ident - ($scalar0:expr, $scalar1:expr)) => {{
        Vec2::new($vec2.x - $scalar0, $vec2.y - $scalar1)
    }};
}
