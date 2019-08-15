use crate::prelude::*;

/*pub fn circle(pos: Vec2<f32>, center: Vec2<f32>, radius: f32) -> bool {
	let pos = pos - center;
	pos.x * pos.x + pos.y * pos.y < radius * radius
}*/

pub fn rect(pos: Vec2<f32>, rect: Vec2<f32>, size: Vec2<f32>) -> bool {
	pos.x >= rect.x && pos.x <= rect.x + size.x &&
	pos.y >= rect.y && pos.y <= rect.y + size.y
}
