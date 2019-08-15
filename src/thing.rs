use crate::prelude::*;
use crate::vertex::*;
use crate::collision::*;

pub trait Thing {
	fn size(&self) -> Vec2<f32>;
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, mouse: Vec2<f32>, drag_from: Option<Vec2<f32>>);
	fn collides(&self, mouse: Vec2<f32>, pos: Vec2<f32>) -> bool {
		rect(mouse, pos, self.size())
	}
}

pub const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
pub const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
pub const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
pub const PURPLE: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
pub const YELLOW: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
pub const DARK_GREEN: [f32; 4] = [0.05, 0.24, 0.06, 1.0];
pub const DULL_RED: [f32; 4] = [0.59, 0.25, 0.25, 1.0];
pub const GREY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
pub const DARK_GREY: [f32; 4] = [0.3, 0.3, 0.3, 1.0];

use self::Class::*;
use self::Element::*;

pub const UNIT_SIZE: Vec2<f32> = Vec2{ x: 0.3, y: 0.45 };
pub const BUTTON_SIZE: Vec2<f32> = Vec2{ x: 0.3, y: 0.1 };

impl Thing for Unit {
	fn size(&self) -> Vec2<f32> {
		UNIT_SIZE
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, mut pos: Vec2<f32>, m: Vec2<f32>, drag_from: Option<Vec2<f32>>) {
		let drag_from = drag_from.unwrap_or(Vec2::zero());
		if self.collides(drag_from, pos) {
			pos += m - drag_from;
		}
		let size = self.size() / vec2(1.0, 3.0);
		let offset = vec2(0.0, size.y);
		let c = match self.class {
			Melee => DULL_RED,
			Ranged => DARK_GREEN,
		};
		quad(v, pos.extend(1.0), size, Color(c));
		let c = match self.element {
			Red => RED,
			Green => GREEN,
			Blue => BLUE,
		};
		quad(v, (pos + offset).extend(1.0), size, Color(c));
		quad(v, (pos + offset * 2.0).extend(1.0), size, Color(PURPLE));
		quad(v, (pos + offset * 2.0).extend(2.0), size * vec2(self.hp / self.max_hp, 1.0).f32(), Color(YELLOW));
		if self.collides(m, pos) {
			let c = [DARK_GREY[0], DARK_GREY[1], DARK_GREY[2], 0.75];
			quad(v2, m.extend(10.0), self.size() * vec2(4.0, 1.0), Color(c));
			let c = match self.class {
				Melee => "melee",
				Ranged => "ranged",
			};
			draw_string(v2, m.extend(11.0), size / 4.0, format!("class: {}",c));
			let c = match self.element {
				Red => "red",
				Green => "green",
				Blue => "blue",
			};
			draw_string(v2, (m + offset).extend(11.0), size / 4.0, format!("element: {}",c));
			draw_string(v2, (m + offset * 2.0).extend(11.0), size / 4.0, format!("hp: {:.2}/{:.2}",self.hp,self.max_hp));
		}
	}
}

impl Thing for UnitView {
	fn size(&self) -> Vec2<f32> {
		UNIT_SIZE
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, m: Vec2<f32>, _drag_from: Option<Vec2<f32>>) {
		let size = self.size() / vec2(1.0, 3.0);
		let offset = vec2(0.0, size.y);
		let c = match self.class {
			Some(Melee) => DULL_RED,
			Some(Ranged) => DARK_GREEN,
			None => GREY,
		};
		quad(v, pos.extend(1.0), size, Color(c));
		let c = match self.element {
			Some(Red) => RED,
			Some(Green) => GREEN,
			Some(Blue) => BLUE,
			None => GREY,
		};
		quad(v, (pos + offset).extend(1.0), size, Color(c));
		if let Some(hp) = self.frac_hp {
			quad(v, (pos + offset * 2.0).extend(1.0), size, Color(PURPLE));
			quad(v, (pos + offset * 2.0).extend(2.0), size * vec2(hp, 1.0).f32(), Color(YELLOW));
		} else {
			quad(v, (pos + offset * 2.0).extend(1.0), size, Color(GREY));
		}
		if self.collides(m, pos) {
			let c = [DARK_GREY[0], DARK_GREY[1], DARK_GREY[2], 0.75];
			quad(v2, m.extend(10.0), self.size() * vec2(4.0, 1.0), Color(c));
			let c = match self.class {
				Some(Melee) => "melee",
				Some(Ranged) => "ranged",
				None => "unknown",
			};
			draw_string(v2, m.extend(11.0), size / 4.0, format!("class: {}",c));
			let c = match self.element {
				Some(Red) => "red",
				Some(Green) => "green",
				Some(Blue) => "blue",
				None => "unknown",
			};
			draw_string(v2, (m + offset).extend(11.0), size / 4.0, format!("element: {}",c));
			draw_string(v2, (m + offset * 2.0).extend(11.0), size / 4.0, format!("hp: {:.2}/{:.2}",self.hp(),self.max_hp()));
		}
	}
}

impl Thing for MoveOption {
	fn size(&self) -> Vec2<f32> {
		BUTTON_SIZE
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, m: Vec2<f32>, _drag_from: Option<Vec2<f32>>) {
		let size = self.size();
		let c = [GREY[0], GREY[1], GREY[2], GREY[3] * if self.collides(m, pos) { 0.7 } else { 1.0 }];
		quad(v, pos.extend(1.0), size, Color(c));
		draw_string(v2, pos.extend(10.0), size * vec2(0.5, 1.0), format!("{}",self.id));
	}
}

pub struct Button {
	pub name: String,
	pub pos: Vec2<f32>,
	pub size: Vec2<f32>,
	pub tex: Tex,
}

impl Thing for Button {
	fn size(&self) -> Vec2<f32> {
		self.size
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, m: Vec2<f32>, _drag_from: Option<Vec2<f32>>) {
		let size = self.size();
		let t = match self.tex {
			Color(c) => Color([c[0], c[1], c[2], c[3] * if self.collides(m, pos) { 0.7 } else { 1.0 }]),
			Texture(_n) => unimplemented!(),
		};
		quad(v, (pos + self.pos).extend(1.0), size, t);
		draw_string(v2, (pos + self.pos + vec2(0.0, self.size.y / 8.0)).extend(10.0), size / size.normalize() * 0.16, self.name.clone());
	}
	
	fn collides(&self, m: Vec2<f32>, p: Vec2<f32>) -> bool {
		rect(m, p + self.pos, self.size())
	}
}
