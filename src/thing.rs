use crate::prelude::*;
use crate::vertex::*;

pub trait Thing {
	type Args;
	fn size(&self, size: Vec2<f32>, args: Self::Args) -> Vec2<f32>;
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, size: Vec2<f32>, mouse: Vec2<f32>, drag_from: Option<Vec2<f32>>, args: Self::Args);
	fn collides(&self, m: Vec2<f32>, pos: Vec2<f32>, size: Vec2<f32>, args: Self::Args) -> Option<usize> {
		if rect(m, pos, self.size(size, args)) { Some(0) } else { None }
	}
	fn select(&self, _args: Self::Args) -> Option<Box<dyn Thing<Args=Self::Args>>> {
		None
	}
}

//pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
pub const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
pub const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
pub const PURPLE: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
pub const YELLOW: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
pub const DARK_GREEN: [f32; 4] = [0.05, 0.24, 0.06, 1.0];
pub const DULL_RED: [f32; 4] = [0.59, 0.25, 0.25, 1.0];
pub const GREY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
pub const DARK_GREY: [f32; 4] = [0.3, 0.3, 0.3, 1.0];
pub const VERY_DARK_GREY: [f32; 4] = [0.1, 0.1, 0.1, 1.0];
pub const CYAN: [f32; 4] = [0.0, 0.8, 0.8, 1.0];
//pub const WHITE: [f32; 4] = [1.0; 4];

pub const PERK_SIZE: f32 = UNIT_SIZE.x / 3.2;
pub const EQUIP_SIZE: f32 = UNIT_SIZE.x / 1.5;
pub const DESC_WIDTH: usize = 20;

use self::Class::*;
use self::Element::*;

pub const UNIT_SIZE: Vec2<f32> = Vec2{ x: 0.3, y: 0.45 };
pub const BUTTON_SIZE: Vec2<f32> = Vec2{ x: 0.3, y: 0.1 };

impl Thing for Unit {
	type Args = bool;
	fn size(&self, size: Vec2<f32>, _heal_button: bool) -> Vec2<f32> {
		UNIT_SIZE * size
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, mut pos: Vec2<f32>, size: Vec2<f32>, m: Vec2<f32>, drag_from: Option<Vec2<f32>>, heal_button: bool) {
		let mut mouseover_shift_left = false;
		drag_from.map(|d| if let Some(0) = self.collides(d, pos, size, false) {
			pos += m - d;
			mouseover_shift_left = true;
		});
		UnitView {
			unit: self.clone(),
			class_revealed: true,
			element_revealed: true,
			frac_hp_revealed: true,
		}.draw(v, v2, pos, size, m, drag_from, mouseover_shift_left);
		if let Some(perks) = self.perk_choice.as_ref() {
			let mut p = pos + vec2(0.0, - perks[0].size(size, false).y * 1.1);
			for perk in perks.iter() {
				perk.draw(v, v2, p, size, m, drag_from, false);
				p += vec2(perk.size(size, false).x * 1.1, 0.0);
			}
		}
		if heal_button {
			let cost = (1.0 - self.hp / self.max_hp) * HEAL_COST * (self.perks.len() + 5 + self.perk_choice.is_some() as usize) as f64 / 5.0;
			if cost > 1e-8 {
				Button {
					name: format!("heal: {:.2}",cost),
					pos: pos + vec2(0.0, self.size(size, heal_button).y + 0.1),
					size: self.size(size, heal_button) / vec2(1.0, 6.0),
					tex: Color(GREEN),
					edge: None,
				}.draw(v, v2, Vec2::zero(), Vec2::one(), m, drag_from, false);
			}
		}
	}
	
	fn collides(&self, m: Vec2<f32>, pos: Vec2<f32>, size: Vec2<f32>, heal_button: bool) -> Option<usize> {
		if rect(m, pos, self.size(size, false)) {
			Some(0)
		} else if rect(m, pos + vec2(0.0, self.size(size, heal_button).y + 0.1), self.size(size, heal_button) / vec2(1.0, 6.0)) && heal_button {
			Some(1)
		} else if let Some(perks) = self.perk_choice.as_ref() {
			let mut p = pos + vec2(0.0, -perks[0].size(size, false).y * 1.1);
			for i in 0..perks.len() {
				if perks[i].collides(m, p, size, false).is_some() {
					return Some(i + 2);
				}
				p += vec2(perks[i].size(size, false).x * 1.1, 0.0);
			}
			None
		} else {
			None
		}
	}
	
	fn select(&self, _heal_button: bool) -> Option<Box<dyn Thing<Args=Self::Args>>> {
		Some(Box::new((self.perks.clone(), self.equipment.clone())))
	}
}

impl Thing for UnitView {
	type Args = bool;
	fn size(&self, size: Vec2<f32>, _: bool) -> Vec2<f32> {
		self.unit.size(size, false)
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, s: Vec2<f32>, mut m: Vec2<f32>, _drag_from: Option<Vec2<f32>>, mouseover_shift_left: bool) {
		let size = self.size(s, false) / vec2(1.0, 3.0);
		let offset = vec2(0.0, size.y);
		let c = match (self.class_revealed, self.class) {
			(true, Melee) => DULL_RED,
			(true, Ranged) => DARK_GREEN,
			(false, _) => GREY,
		};
		quad(v, pos.extend(1.0), size, Color(c));
		let c = match (self.element_revealed, self.element) {
			(true, Red) => RED,
			(true, Green) => GREEN,
			(true, Blue) => BLUE,
			(false, _) => GREY,
		};
		quad(v, (pos + offset).extend(1.0), size, Color(c));
		quad(v, (pos + offset * 2.0).extend(1.0), size, Color(if self.frac_hp_revealed { DARK_GREY } else { GREY }));
		if self.frac_hp_revealed {
			quad(v, (pos + offset * 2.0).extend(2.0), size * vec2(self.hp_lim / self.max_hp, 1.0).f32(), Color(PURPLE));
			quad(v, (pos + offset * 2.0).extend(2.0), size * vec2(self.hp / self.max_hp, 1.0).f32(), Color(YELLOW));
		}
		let mut perks = String::new();
		(0..self.perks.len()).map(|i| if i % 10 == 0 { perks.push('\n'); perks.push('*') } else { perks.push('*') }).last();
		draw_string(v2, (pos + offset * 3.0).extend(2.0), vec2(0.03, 0.03) * s, &perks, None);
		if let Some(0) = self.collides(m, pos, s, false) {
			if (m + self.size(Vec2::one(), false) * vec2(4.0, 1.0)).y > top_edge() {
				m.y -= (self.size(Vec2::one(), false) * vec2(4.0, 1.0)).y;
			}
			if (m + self.size(Vec2::one(), false) * vec2(4.0, 1.0)).x > right_edge() || mouseover_shift_left {
				m.x -= (self.size(Vec2::one(), false) * vec2(4.0, 1.0)).x;
			}
			let c = [DARK_GREY[0], DARK_GREY[1], DARK_GREY[2], 0.75];
			quad(v2, m.extend(10.0), self.size(Vec2::one(), false) * vec2(4.0, 1.0), Color(c));
			let c = match (self.class_revealed, self.class) {
				(true, Melee) => "melee",
				(true, Ranged) => "ranged",
				(false, _) => "??",
			};
			let size = size / s / 3.5;
			let offset = vec2(0.0,size.y);
			draw_string(v2, m.extend(11.0), size, &format!("class: {}",c), None);
			let c = match (self.element_revealed, self.element) {
				(true, Red) => "red",
				(true, Green) => "green",
				(true, Blue) => "blue",
				(false, _) => "??",
			};
			draw_string(v2, (m + offset).extend(11.0), size, &format!("element: {}",c), None);
			draw_string(v2, (m + offset * 2.0).extend(11.0), size, &format!("perks: {}",self.perks.len()), None);
			draw_string(v2, (m + offset * 3.0).extend(11.0), size, &format!("attack: {:.2}",self.attack), None);
			draw_string(v2, (m + offset * 4.0).extend(11.0), size, &format!("armor: {:.2}",self.armor), None);
			draw_string(v2, (m + offset * 5.0).extend(11.0), size, &format!("block: {:.2}",self.block), None);
			draw_string(v2, (m + offset * 6.0).extend(11.0), size, &format!("regen: {:.2}",self.regen), None);
			draw_string(v2, (m + offset * 7.0).extend(11.0), size, &if self.frac_hp_revealed { format!("hp: {:.2}/{:.2}",self.hp,self.max_hp) } else { format!("hp: ??/{:.2}",self.max_hp) }, None);
		}
	}
	
	fn collides(&self, m: Vec2<f32>, pos: Vec2<f32>, size: Vec2<f32>, _: bool) -> Option<usize> {
		self.unit.collides(m, pos, size, false)
	}
	
	fn select(&self, _: bool) -> Option<Box<dyn Thing<Args=Self::Args>>> {
		self.unit.select(false)
	}
}

impl Thing for Perk {
	type Args = bool;
	fn size(&self, size: Vec2<f32>, _: bool) -> Vec2<f32> {
		vec2(PERK_SIZE, PERK_SIZE) * size
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, size: Vec2<f32>, m: Vec2<f32>, _drag_from: Option<Vec2<f32>>, _: bool) {
		quad(v, pos.extend(12.0), self.size(size, false), Color([self.color[0], self.color[1], self.color[2], 1.0]));
		if self.collides(m, pos, size, false).is_some() {
			draw_perk_mouseover(self, v2, m.extend(13.0));
		}
	}
}

fn draw_perk_mouseover(p: &Perk, v2: &mut Vec<Vertex>, pos: Vec3<f32>) {
	draw_string(v2, pos, vec2(p.size(Vec2::one(), false).x, p.size(Vec2::one(), false).x) * 0.5, &p.desc, Some((Color([p.color[0] * 0.8, p.color[1] * 0.8, p.color[2] * 0.8, 0.7]), DESC_WIDTH)));
}

impl Thing for Equipment {
	type Args = bool;
	fn size(&self, size: Vec2<f32>, drag: bool) -> Vec2<f32> {
		let x: Option<Self> = None;
		x.size(size, drag) * 0.9
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, mut pos: Vec2<f32>, s: Vec2<f32>, m: Vec2<f32>, drag_from: Option<Vec2<f32>>, drag: bool) {
		let mut mouseover_shift_left = false;
		if drag { drag_from.map(|d| if let Some(0) = self.collides(d, pos, s, false) {
			pos += m - d;
			mouseover_shift_left = true;
		}); }
		let size = self.size(s, drag);
		quad(v, pos.extend(10.0), size, Color([self.color[0], self.color[1], self.color[2], 1.0]));
		if self.collides(m, pos, s, false).is_some() {
			draw_equip_mouseover(self, v2, m.extend(12.0), mouseover_shift_left);
		}
	}
}

impl Thing for Option<Equipment> {
	type Args = bool;
	fn size(&self, size: Vec2<f32>, _drag: bool) -> Vec2<f32> {
		vec2(EQUIP_SIZE, EQUIP_SIZE) * size
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, size: Vec2<f32>, m: Vec2<f32>, drag_from: Option<Vec2<f32>>, drag: bool) {
		quad(v, pos.extend(1.0), self.size(size, drag), Color(DARK_GREY));
		self.as_ref().map(|e| e.draw(v, v2, pos + self.size(size, drag) * 0.05, size, m, drag_from, drag));
	}
}

fn draw_equip_mouseover(e: &Equipment, v2: &mut Vec<Vertex>, mut pos: Vec3<f32>, mouseover_shift_left: bool) {
	let size = e.size(Vec2::one(), false) * 0.3;
	let size = vec2(size.x, size.x);
	if pos.x + size.x * DESC_WIDTH as f32 > right_edge() || mouseover_shift_left {
		pos.x -= size.x * DESC_WIDTH as f32;
	}
	let mut n = if e.desc.is_empty() { 0.0 } else {
		e.desc.chars().map(|c| if c == '\n' { 1.0 } else { 0.0 }).sum::<f32>() + 1.0
	} + e.stat_name1_secondary().is_some() as u8 as f32 + e.stat_name2_secondary().is_some() as u8 as f32;
	if pos.y + size.y * (n+3.0) > top_edge() {
		pos.y -= size.y * (n+3.0);
	}
	let offset = vec3(0.0, size.y, 0.0);
	let c = Some((Color([e.color[0] * 0.8, e.color[1] * 0.8, e.color[2] * 0.8, 0.7]), DESC_WIDTH));
	if !e.desc.is_empty() {
		draw_string(v2, pos, size, &e.desc, c);
	}
	draw_string(v2, pos + offset * n, size, &format!("repair_cost: {:.3}",e.repair_cost), c);
	draw_string(v2, pos + offset * (n+1.0), size, &format!("durability: {:.3}",e.durability), c);
	e.stat_name2_secondary().map(|s| {
		draw_string(v2, pos + offset * (n+2.0), size, &format!("{}: {:.3}",s,e.stat2.1), c);
		n += 1.0;
	});
	draw_string(v2, pos + offset * (n+2.0), size, &format!("{}: {:.3}",e.stat_name2(),e.stat2.0), c);
	e.stat_name1_secondary().map(|s| {
		draw_string(v2, pos + offset * (n+3.0), size, &format!("{}: {:.3}",s,e.stat1.1), c);
		n += 1.0;
	});
	draw_string(v2, pos + offset * (n+3.0), size, &format!("{}: {:.3}",e.stat_name1(),e.stat1.0), c);
}

impl Thing for (Vec<Perk>, [Option<Equipment>; 4]) {
	type Args = bool;
	fn size(&self, size: Vec2<f32>, _: bool) -> Vec2<f32> {
		size
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, s: Vec2<f32>, m: Vec2<f32>, drag_from: Option<Vec2<f32>>, drag: bool) {
		let mut p = pos + 0.01;
		for i in 0..self.0.len() {
			self.0[i].draw(v, v2, p, s, m, drag_from, drag);
			p.x += self.0[i].size(s, drag).x * 1.1;
		}
		p = pos + 0.01;
		p.y += PERK_SIZE * 1.1;
		for e in self.1.iter() {
			e.draw(v, v2, p, s, m, drag_from, drag);
			p.x += e.size(s, drag).x * 1.1;
		}
	}
	
	fn collides(&self, m: Vec2<f32>, pos: Vec2<f32>, s: Vec2<f32>, collide_slots: bool) -> Option<usize> {
		let mut p = pos + 0.01;
		p.y += PERK_SIZE * 1.1;
		for i in 0..self.1.len() {
			if collide_slots {
				if self.1[i].collides(m, p, s, false).is_some() {
					return Some(i)
				}
			} else {
				if self.1[i].as_ref().and_then(|e| e.collides(m, p, s, false)).is_some() {
					return Some(i)
				}
			}
			p.x += self.1[i].size(s, false).x * 1.1;
		}
		None
	}
}

impl Thing for MoveOption {
	type Args = bool;
	fn size(&self, size: Vec2<f32>, _: bool) -> Vec2<f32> {
		BUTTON_SIZE * size
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, pos: Vec2<f32>, s: Vec2<f32>, m: Vec2<f32>, _drag_from: Option<Vec2<f32>>, _: bool) {
		let size = self.size(s, false);
		let c = [GREY[0], GREY[1], GREY[2], GREY[3] * if self.collides(m, pos, s, false).is_some() { 0.6 } else { 1.0 }];
		quad(v, pos.extend(1.0), size, Color(c));
		let offset = vec2(0.0, size.y * 0.5);
		let pos = pos + vec2(0.0, size.y);
		let size = vec2(size.y, size.y) * 0.45;
		self.max_group_size.map(|x| draw_string(v2, (pos - offset).extend(10.0), size, &format!("{}",x), None));
		draw_string(v2, (pos - offset * 2.0).extend(10.0), size, &self.name, None);
	}
}

pub struct Button {
	pub name: String,
	pub pos: Vec2<f32>,
	pub size: Vec2<f32>,
	pub tex: Tex,
	pub edge: Option<bool>, //specifies whether x position is absolute or relative to left/right edge
}

impl Thing for Button {
	type Args = bool;
	fn size(&self, _: Vec2<f32>, _: bool) -> Vec2<f32> {
		self.size
	}
	
	fn draw(&self, v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, _: Vec2<f32>, _: Vec2<f32>, m: Vec2<f32>, drag_from: Option<Vec2<f32>>, drag: bool) {
		let size = self.size(Vec2::one(), false);
		let t = match self.tex {
			Color(c) => Color([c[0], c[1], c[2], c[3] * if self.collides(m, Vec2::zero(), Vec2::one(), false).is_some() { 0.6 } else { 1.0 }]),
			Texture(n) => Texture(n),
			Blend(c, n, blend) => Blend([c[0], c[1], c[2], c[3] * if self.collides(m, Vec2::zero(), Vec2::one(), false).is_some() { 0.6 } else { 1.0 }], n, blend),
		};
		let mut pos = self.pos();
		drag_from.map(|d| if drag && self.collides(d, Vec2::zero(), Vec2::one(), false).is_some() { pos += m - d });
		quad(v, pos.extend(1.0), size, t);
		let char_size = ((size.x * 0.9) / self.name.len() as f32).min(size.y * 0.9);
		draw_string(v2, (pos + vec2((size.x - self.name.len() as f32 * char_size) * 0.5, (size.y - char_size) / 2.0)).extend(10.0), vec2(char_size, char_size), &self.name, None);
	}
	
	fn collides(&self, m: Vec2<f32>, _: Vec2<f32>, _: Vec2<f32>, _: bool) -> Option<usize> {
		if rect(m, self.pos(), self.size(Vec2::one(), false)) { Some(0) } else { None }
	}
}

impl Button {
	pub fn edgeified(mut self, e: bool) -> Self {
		self.pos.x -= if e { left_edge() } else { right_edge() };
		self.edge = Some(e);
		self
	}
	
	pub fn pos(&self) -> Vec2<f32> {
		self.pos + self.edge.map(|e| if e { vec2(left_edge(), 0.0) } else { vec2(right_edge(), 0.0) }).unwrap_or(Vec2::zero())
	}
}
