#![feature(const_transmute)]
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MouseEvent,WebSocket,ErrorEvent,HtmlInputElement,MessageEvent,KeyboardEvent};
use std::cell::Cell;

macro_rules! log {
	( $( $t:tt )* ) => {
		web_sys::console::log_1(&format!( $( $t )* ).into());
	}
}

mod vertex;
use vertex::*;
mod prelude;
use prelude::*;
mod boiler_plate;
use boiler_plate::*;
mod thing;
use thing::*;
mod collision;

const DEFAULT_IP: &str = "192.168.1.55";
const MESSAGE_DURATION: f32 = 30.0;
const FRAMES_PER_SNAPSHOT: usize = 1;

const TEXT_SIZE: Vec2<f32> = Vec2{ x: 0.06, y: 0.06 };

#[derive(Debug,Clone)]
enum State {
	SafeZone(Vec<Unit>, Vec<Equipment>),
	Looting,
	InQueue,
	InFight(bool),
}
use State::*;

#[allow(unused)]
impl State {
	fn is_safe_zone(&self) -> bool { if let SafeZone(_,_) = self { true } else { false } }
	fn is_looting(&self) -> bool { if let Looting = self { true } else { false } }
	fn is_in_queue(&self) -> bool { if let InQueue = self { true } else { false } }
	fn is_in_fight(&self) -> bool { if let InFight(_) = self { true } else { false } }
	fn storage(&self) -> Option<(&Vec<Unit>, &Vec<Equipment>)> { if let SafeZone(ref u, ref e) = self { Some((u, e)) } else { None } }
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	console_error_panic_hook::set_once();
	
	let document = window().document().expect(l!());
	let canvas = Rc::new(document.get_element_by_id("canvas").expect(l!()).dyn_into::<web_sys::HtmlCanvasElement>()?);
	let context = setup_rendering(&canvas, &document)?;
	let ip = window().location().href().map_err(|_| ()).and_then(|s| {
		let mut s = s.split('?'); s.next();	s.next().and_then(|s| s.find("ip=").map(|i| {	let s = s.split_at(i+3).1; s.split('&').next().expect(l!()).to_string()	})).ok_or(())	}).unwrap_or(DEFAULT_IP.to_string());
	let ws = WebSocket::new(&format!("ws://{}:2794",ip))?;
	
	let depth = Rc::new(Cell::new(0usize));
	let gold = Rc::new(Cell::new(0f64));
	let juice = Rc::new(Cell::new(0f64));
	let repair_button_selected = Rc::new(Cell::new(false));
	let repair_target = Rc::new(Cell::new(5f64));
	let equipment: Rc<RefCell<Vec<Equipment>>> = Rc::new(RefCell::new(Vec::new()));
	let recording: Rc<RefCell<Option<(FightRecording, usize, bool)>>> = Rc::new(RefCell::new(None));
	let state: Rc<RefCell<State>> = Rc::new(RefCell::new(InQueue));
	let selected: Rc<RefCell<Option<(Box<dyn Thing<Args=bool>>, Option<InventoryType>)>>> = Rc::new(RefCell::new(None));
	let messages: Rc<RefCell<Vec<(String, f32)>>> = Rc::new(RefCell::new(Vec::new()));
	let move_options: Rc<RefCell<Vec<MoveOption>>> = Rc::new(RefCell::new(Vec::new()));
	let team: Rc<RefCell<Vec<Unit>>> = Rc::new(RefCell::new(Vec::new()));
	let opponent: Rc<RefCell<Vec<UnitView>>> = Rc::new(RefCell::new(Vec::new()));
	let opponent_name = Rc::new(RefCell::new(ArrayString::new()));
	
	let fight_button = Rc::new(Button {
		name: "fight".to_string(),
		pos: vec2(-0.5, 0.0) - BUTTON_SIZE * vec2(2.5, 1.5) * 0.5,
		size: BUTTON_SIZE * vec2(2.5, 1.5),
		tex: Color(RED),
		edge: None,
	});
	let do_not_button = Rc::new(Button {
		name: "do not".to_string(),
		pos: vec2(0.5, 0.0) - BUTTON_SIZE * vec2(2.5, 1.5) * 0.5,
		size: BUTTON_SIZE * vec2(2.5, 1.5),
		tex: Color(GREEN),
		edge: None,
	});
	let up_button = Rc::new(Button {
		name: "up".to_string(),
		pos: vec2(-BUTTON_SIZE.x * 1.25, 0.025 + BUTTON_SIZE.y * 0.75),
		size: BUTTON_SIZE * vec2(2.5, 1.5),
		tex: Color(YELLOW),
		edge: None,
	});
	let stay_button = Rc::new(Button {
		name: "stay".to_string(),
		pos: vec2(-BUTTON_SIZE.x * 1.25, -BUTTON_SIZE.y * 0.75),
		size: BUTTON_SIZE * vec2(2.5, 1.5),
		tex: Color(DARK_GREY),
		edge: None,
	});
	let down_button = Rc::new(Button {
		name: "down".to_string(),
		pos: vec2(-BUTTON_SIZE.x * 1.25, -0.025 - BUTTON_SIZE.y * 2.25),
		size: BUTTON_SIZE * vec2(2.5, 1.5),
		tex: Color(PURPLE),
		edge: None,
	});
	let purchase_unit_button = Rc::new(Button {
		name: format!("purchase unit: {:.2}",UNIT_COST),
		pos: -BUTTON_SIZE * vec2(1.25, 0.5) + vec2(0.0, bottom_edge() + 0.07),
		size: BUTTON_SIZE * vec2(2.5, 1.0),
		tex: Color(CYAN),
		edge: None,
	});
	let pause_button = Rc::new(RefCell::new(Button {
		name: "pause".to_string(),
		pos: -vec2(BUTTON_SIZE.y, BUTTON_SIZE.y) * 0.5,
		size: vec2(BUTTON_SIZE.y, BUTTON_SIZE.y),
		tex: Color(GREEN),
		edge: None,
	}));
	let rewind_button = Rc::new(Button {
		name: "rewind".to_string(),
		pos: -vec2(BUTTON_SIZE.y, BUTTON_SIZE.y) * 0.5 - vec2(BUTTON_SIZE.y * 1.1, 0.0),
		size: vec2(BUTTON_SIZE.y, BUTTON_SIZE.y),
		tex: Color(RED),
		edge: None,
	});
	let skip_button = Rc::new(Button {
		name: "skip".to_string(),
		pos: -vec2(BUTTON_SIZE.y, BUTTON_SIZE.y) * 0.5 + vec2(BUTTON_SIZE.y * 1.1, 0.0),
		size: vec2(BUTTON_SIZE.y, BUTTON_SIZE.y),
		tex: Color(BLUE),
		edge: None,
	});
	let heal_all_button = Rc::new(Button {
		name: "heal all".to_string(),
		pos: vec2(0.0, -0.02) - BUTTON_SIZE * 0.75,
		size: BUTTON_SIZE * 1.5,
		tex: Color(GREEN),
		edge: None,
	});
	let square_button_size = vec2(BUTTON_SIZE.y, BUTTON_SIZE.y) * 1.5;
	let repair_button = Rc::new(RefCell::new(Button {
		name: format!("{:.3}",repair_target.get()),
		pos: equip_box_pos() + equip_box_size() - vec2(square_button_size.x * 2.0 + 0.02, -0.02),
		size: square_button_size,
		tex: Color(CYAN),
		edge: None,
	}.edgeified(false)));
	let juice_button = Rc::new(Button {
		name: "juice".to_string(),
		pos: equip_box_pos() + equip_box_size() - vec2(square_button_size.x, -0.02),
		size: square_button_size,
		tex: Color(RED),
		edge: None,
	}.edgeified(false));
	
	let next_click = Rc::new(Cell::new(false));
	let next_click2 = next_click.clone();
	let repair_button_selected2 = repair_button_selected.clone();
	let repair_target2 = repair_target.clone();
	let repair_button2 = repair_button.clone();
	let canvas2 = canvas.clone();
	let fight_button2 = fight_button.clone();
	let do_not_button2 = do_not_button.clone();
	let up_button2 = up_button.clone();
	let stay_button2 = stay_button.clone();
	let down_button2 = down_button.clone();
	let pause_button2 = pause_button.clone();
	let rewind_button2 = rewind_button.clone();
	let skip_button2 = skip_button.clone();
	let move_options2 = move_options.clone();
	let heal_all_button2 = heal_all_button.clone();
	let team2 = team.clone();
	let opponent2 = opponent.clone();
	let ws2 = ws.clone();
	let selected2 = selected.clone();
	let state2 = state.clone();
	let purchase_unit_button2 = purchase_unit_button.clone();
	let recording2 = recording.clone();
	let depth2 = depth.clone();
	let gold2 = gold.clone();
	let onclick = Closure::wrap(Box::new(move|e: MouseEvent| {
		repair_button_selected2.set(false);
		repair_button2.borrow_mut().tex = Color(CYAN);
		repair_button2.borrow_mut().name = format!("{:.3}",repair_target2.get());
		if next_click2.replace(true) {
			let m = screen_coords(e.client_x(), e.client_y(), &canvas2);
			let mut selected = selected2.borrow_mut();
			let mut clicked = false;
			let mut state = state2.borrow_mut();
			let mut recording = recording2.borrow_mut();
			if recording.is_some() && skip_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
				clicked = true;
				*recording = None;
			} else if let Some(ref mut r) = &mut*recording {
				if pause_button2.borrow().collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
					clicked = true;
					r.2 = !r.2;
					pause_button2.borrow_mut().tex = Color(if r.2 {
						DARK_GREEN
					} else {
						GREEN
					});
				} else if rewind_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
					clicked = true;
					r.1 = 0;
				}
				let (t, o) = &r.0.get(r.1);
				if !t.is_empty() {
					let size = t[0].size(Vec2::one(), false);
					for i in 0..t.len() {
						if let Some(x) = t[i].collides(m, team_unit_pos(t.len(), i, size), Vec2::one(), state.is_safe_zone()) {
							clicked = true;
							if x == 0 {
								*selected = t[i].select(false).map(|s| (s, Some(InventoryType::Team(i))));
							} else if x == 1 {
								send(&ws2, ClientPacket::Purchase(i+1)).expect(l!());
							} else {
								send(&ws2, ClientPacket::PerkChoice(i, x - 2)).expect(l!());
							}
							break;
						}
					}
				}
				if !o.is_empty() {
					let size = o[0].size(Vec2::one(), false);
					for i in 0..o.len() {
						if o[i].collides(m, opponent_unit_pos(o.len(), i, size), Vec2::one(), false).is_some() {
							clicked = true;
							*selected = o[i].select(false).map(|s| (s, None));
							break;
						}
					}
				}
			} else {
				match &mut*state {
					SafeZone(unit_storage, _equipment_storage) => {
						let mut r = repair_button2.borrow_mut();
						if r.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
							r.tex = Color([CYAN[0] * 1.1, CYAN[1] * 1.1, CYAN[2] * 1.1, CYAN[3]]);
							r.name = "".into();
							repair_button_selected2.set(true);
						}
						if !unit_storage.is_empty() {
							let size = unit_storage[0].size(storage_unit_scale(), false);
							for i in 0..unit_storage.len() {
								if let Some(0) = unit_storage[i].collides(m, storage_unit_pos(i, size) + unit_storage_box_pos(), storage_unit_scale(), false) {
									clicked = true;
									*selected = unit_storage[i].select(false).map(|s| (s, Some(InventoryType::UnitStorage(i))));
									break;
								}
							}
						}
						let mut mo = move_options2.borrow_mut();
						if !mo.is_empty() {
							let button_start = vec2(-(4.5 * 0.1 + 5.0), (mo.len() / 10) as f32 * 0.55 + 0.5) * mo[0].size(Vec2::one(), false);
							let mut p = button_start;
							let step = mo[0].size(Vec2::one(), false) * 1.1;
							for i in 0..mo.len() {
								if mo[i].collides(m, p, Vec2::one(), false).is_some() && team2.borrow().len() <= mo[i].max_group_size.unwrap_or(usize::max_value()) && !team2.borrow().is_empty() {
									clicked = true;
									send(&ws2, ClientPacket::Move(i)).expect(l!());
									gold2.set(0.0);
									depth2.set(1);
									*mo = Vec::new();
									*state = InQueue;
									break;
								}
								if (i+1) % 10 == 0 {
									p.y -= step.y;
									p.x = button_start.x;
								} else {
									p.x += step.x;
								}
							}
						}
						if purchase_unit_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
								clicked = true;
							send(&ws2, ClientPacket::Purchase(0)).expect(l!());
						}
						if heal_all_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
							clicked = true;
							for i in 0..team2.borrow().len() {
								send(&ws2, ClientPacket::Purchase(i+1)).expect(l!());
							}
						}
					},
					Looting => {
						if up_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
							clicked = true;
							send(&ws2, ClientPacket::Move(0)).expect(l!());
							depth2.set(depth2.get() - 1);
							*state = InQueue;
						} else if stay_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
							clicked = true;
							send(&ws2, ClientPacket::Move(1)).expect(l!());
							*state = InQueue;
						} else if down_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
							clicked = true;
							send(&ws2, ClientPacket::Move(2)).expect(l!());
							depth2.set(depth2.get() + 1);
							*state = InQueue;
						}
					},
					InQueue => {
						
					},
					InFight(ref mut chosen) => {
						if !*chosen {
							if fight_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
								clicked = true;
								send(&ws2, ClientPacket::Fight(true)).expect(l!());
							} else if do_not_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
								clicked = true;
								send(&ws2, ClientPacket::Fight(false)).expect(l!());
								*chosen = true;
							}
						}
					},
				}
				let t = team2.borrow();
				if !t.is_empty() {
					for i in 0..t.len() {
						if let Some(x) = t[i].collides(m, team_unit_pos(t.len(), i, t[0].size(Vec2::one(), false)), Vec2::one(), state.is_safe_zone()) {
							clicked = true;
							if x == 0 {
								*selected = t[i].select(false).map(|s| (s, Some(InventoryType::Team(i))));
							} else if x == 1 {
								send(&ws2, ClientPacket::Purchase(i+1)).expect(l!());
							} else {
								send(&ws2, ClientPacket::PerkChoice(i, x - 2)).expect(l!());
							}
							break;
						}
					}
				}
				let o = opponent2.borrow();
				if !o.is_empty() {
					let mut p = vec2((-(o.len() as f32 / 2.0) - 0.1) * o[0].size(Vec2::one(), false).x, 0.3);
					for i in 0..o.len() {
						if o[i].collides(m, p, Vec2::one(), false).is_some() {
							clicked = true;
							*selected = o[i].select(false).map(|s| (s, None));
							break;
						}
						p += vec2(o[i].size(Vec2::one(), false).x * 1.1, 0.0);
					}
				}
			}
			if !clicked {
				*selected = None;
			}
		}
	}) as Box<dyn Fn(_)>);
	canvas.set_onclick(Some(onclick.as_ref().unchecked_ref()));
	onclick.forget();
	
	let drag_pos = Rc::new(Cell::new(None));
	let a = drag_pos.clone();
	let b = a.clone();
	let c = a.clone();
	let canvas2 = canvas.clone();
	let onmousedown = Closure::wrap(Box::new(move|e: MouseEvent| if e.button() == 0 { a.set(Some(screen_coords(e.client_x(), e.client_y(), &canvas2))) })
		as Box<dyn Fn(_)>);
	canvas.set_onmousedown(Some(onmousedown.as_ref().unchecked_ref()));
	onmousedown.forget();
	
	let repair_target2 = repair_target.clone();
	let repair_button2 = repair_button.clone();
	let juice_button2 = juice_button.clone();
	let team2 = team.clone();
	let canvas2 = canvas.clone();
	let ws2 = ws.clone();
	let state2 = state.clone();
	let equipment2 = equipment.clone();
	let selected2 = selected.clone();
	let onmouseup = Closure::wrap(Box::new(move|e: MouseEvent| if e.button() == 0 {
		if let Some(d) = b.replace(None) {
			let t = team2.borrow();
			let eq = equipment2.borrow();
			let state = state2.borrow();
			let m = screen_coords(e.client_x(), e.client_y(), &canvas2);
			next_click.set((d - m).magnitude() < 2e-2);
			let mut from = None;
			let mut to = None;
			for i in 0..t.len() {
				if let Some(0) = t[i].collides(d, team_unit_pos(t.len(), i, t[i].size(Vec2::one(), false)), Vec2::one(), false) {
					from = Some(InventoryType::Team(i));
				}
				if let Some(0) = t[i].collides(m, team_unit_pos(t.len(), i, t[i].size(Vec2::one(), false)), Vec2::one(), false) {
					to = Some(InventoryType::Team(i));
				}
			}
			for i in 0..eq.len() {
				if eq[i].collides(d, equip_pos(i) + equip_box_pos(), equip_scale(), false).is_some() {
					from = Some(InventoryType::EquipmentStorage(false, i));
				}
				if eq[i].collides(m, equip_pos(i) + equip_box_pos(), equip_scale(), false).is_some() {
					to = Some(InventoryType::EquipmentStorage(false, i));
				}
			}
			state.storage().map(|(us, es)| {
				for i in 0..us.len() {
					if let Some(0) = us[i].collides(d, storage_unit_pos(i, us[0].size(storage_unit_scale(), false)) + unit_storage_box_pos(), storage_unit_scale(), false) {
						from = Some(InventoryType::UnitStorage(i));
					}
					if let Some(0) = us[i].collides(m, storage_unit_pos(i, us[0].size(storage_unit_scale(), false)) + unit_storage_box_pos(), storage_unit_scale(), false) {
						to = Some(InventoryType::UnitStorage(i));
					}
				}
				for i in 0..es.len() {
					if es[i].collides(d, equip_pos(i) + safe_equip_box_pos(), equip_scale(), false).is_some() {
						from = Some(InventoryType::EquipmentStorage(true, i));
					}
					if es[i].collides(m, equip_pos(i) + safe_equip_box_pos(), equip_scale(), false).is_some() {
						to = Some(InventoryType::EquipmentStorage(true, i));
					}
				}
			});
			let s = selected2.borrow();
			if let Some((s, Some(i))) = &*s {
				use InventoryType::*;
				match *i {
					Team(uidx) => {
						if let Some(i) = s.collides(d, vec2(left_edge(), bottom_edge()), Vec2::one(), false) {
							from = Some(InventoryType::Unit{ in_team: true, uidx, eidx: EquipType::from_idx(i) });
						}
						if let Some(i) = s.collides(m, vec2(left_edge(), bottom_edge()), Vec2::one(), true) {
							to = Some(InventoryType::Unit{ in_team: true, uidx, eidx: EquipType::from_idx(i) });
						}
					},
					UnitStorage(uidx) => {
						if let Some(i) = s.collides(d, vec2(left_edge(), bottom_edge()), Vec2::one(), false) {
							from = Some(InventoryType::Unit{ in_team: false, uidx, eidx: EquipType::from_idx(i) });
						}
						if let Some(i) = s.collides(m, vec2(left_edge(), bottom_edge()), Vec2::one(), true) {
							to = Some(InventoryType::Unit{ in_team: false, uidx, eidx: EquipType::from_idx(i) });
						}
					},
					_ => unimplemented!(),
				}
			}
			let r = repair_button2.borrow();
			if r.collides(d, Vec2::zero(), Vec2::one(), false).is_some() {
				to.map(|to| send(&ws2, ClientPacket::Repair(repair_target2.get(), to)).expect(l!()));
			} else if juice_button2.collides(m, Vec2::zero(), Vec2::one(), false).is_some() {
				from.map(|from| send(&ws2, ClientPacket::Juice(from)).expect(l!()));
			} else {
				let u_size = if t.is_empty() {
					state.storage().map(|(unit_storage, _equipment_storage)| unit_storage[0].size(Vec2::one(), false)).unwrap_or(Vec2::zero())
				} else { t[0].size(Vec2::one(), false) } * 0.2 * 1.1;
				let x: Option<Equipment> = None;
				let s = vec2(0.4, 0.4);
				let size = x.size(s, true) * 10.0;
				if state.is_safe_zone() && to.is_none() {
					if rect(m, vec2(left_edge() + 0.06, 0.84) - vec2(0.0, u_size.y * 6.0), u_size * vec2(14.0, 7.0)) {
						let l = state.storage().unwrap().0.len() - if let Some(InventoryType::UnitStorage(_)) = from { 1 } else { 0 };
						to = Some(InventoryType::UnitStorage(l));
					} else if rect(m, vec2(right_edge() - 0.06, top_edge() - 0.06) - size, size) {
						let l = state.storage().unwrap().1.len() - if let Some(InventoryType::EquipmentStorage(true, _)) = from { 1 } else { 0 };
						to = Some(InventoryType::EquipmentStorage(true, l));
					}
				}
				if to.is_none() && rect(m, vec2(right_edge() - 0.06, bottom_edge() + 0.06) + vec2(-size.x, 0.0), size) {
					let l = eq.len() - if let Some(InventoryType::EquipmentStorage(false, _)) = from { 1 } else { 0 };
					to = Some(InventoryType::EquipmentStorage(false, l));
				}
				if to.is_none() {
					if let Some(InventoryType::Team(_)) = from {
						let x = m.x / (t[0].size(Vec2::one(), false).x * 1.1) + if t.len() % 2 == 0 { 2.0 } else { 1.5 };
						log!("{}",x);
						let toidx = (x.floor() as isize).max(0) as usize;
						to = Some(InventoryType::Team(toidx.min(t.len()-1)));
					} else if let Some(InventoryType::UnitStorage(_)) = from {
						let x = m.x / (state.storage().unwrap().0[0].size(Vec2::one(), false).x * 1.1) + if t.len() % 2 == 0 { 2.5 } else { 2.0 };
						let i = x.floor().max(0.0) as usize;
						log!("{}",x);
						to = Some(InventoryType::Team(i.min(t.len())));
					}
				}
				log!("{:?},{:?}",from,to);
				from.map(|from| to.map(|to|	send(&ws2, ClientPacket::Transfer(from, to)).expect(l!())));
			}
		}
	}) as Box<dyn Fn(_)>);
	canvas.set_onmouseup(Some(onmouseup.as_ref().unchecked_ref()));
	onmouseup.forget();
	
	let canvas2 = canvas.clone();
	let mouse = Rc::new(Cell::new(Vec2::zero()));
	let mouse2 = mouse.clone();
	let onmove = Closure::wrap(Box::new(move|e: MouseEvent| {
			let pos = screen_coords(e.client_x(), e.client_y(), &canvas2);
			mouse2.set(pos);
		if let Some(_initial_pos) = c.get() {
			let _delta = vec2(e.movement_x(), -e.movement_y()).f32() * 2.0 / canvas2.client_height() as f32;
			//something.borrow_mut().drag(initial_pos, pos, delta);
		}
	}) as Box<dyn Fn(_)>);
	canvas.set_onmousemove(Some(onmove.as_ref().unchecked_ref()));
	onmove.forget();
	
	let repair_button2 = repair_button.clone();
	let repair_target2 = repair_target.clone();
	let repair_button_selected2 = repair_button_selected.clone();
	let onkeydown = Closure::wrap(Box::new(move|k: KeyboardEvent| {
		if repair_button_selected2.get() {
			repair_button2.borrow_mut().name.push_str(&k.key());
			if let Ok(n) = repair_button2.borrow().name.parse() {
				repair_target2.set(n);
			}
		}
	}) as Box<dyn Fn(_)>);
	document.set_onkeydown(Some(onkeydown.as_ref().unchecked_ref()));
	onkeydown.forget();
	
	let f = Rc::new(RefCell::new(None));
	let g = f.clone();
	let h = g.clone();
	
	let juice2 = juice.clone();
	let document2 = document.clone();
	let messages2 = messages.clone();
	let equipment2 = equipment.clone();
	let team2 = team.clone();
	let move_options2 = move_options.clone();
	let ws2 = ws.clone();
	let opponent2 = opponent.clone();
	let opponent_name2 = opponent_name.clone();
	let recording2 = recording.clone();
	let selected2 = selected.clone();
	let state2 = state.clone();
	let depth2 = depth.clone();
	let gold2 = gold.clone();
	let onmessage = Closure::wrap(Box::new(move|e: MessageEvent| {
		use self::ServerPacket::*;
		let juice = juice2.clone();
		let document = document2.clone();
		let ws = ws2.clone();
		let h = h.clone();
		let e2 = e.clone();
		let messages = messages2.clone();
		let equipment = equipment2.clone();
		let team = team2.clone();
		let move_options = move_options2.clone();
		let opponent = opponent2.clone();
		let opponent_name = opponent_name2.clone();
		let recording = recording2.clone();
		let selected = selected2.clone();
		let state = state2.clone();
		let depth = depth2.clone();
		let gold = gold2.clone();
		recv(&e, move|p| match p {
			Message(m) => {
				document.get_element_by_id("login result").map(|r| r.set_inner_html(&m));
			},
			_ => {
				let _ = document.get_element_by_id("login box").map(|login_box| login_box.parent_node().expect(l!()).remove_child(&login_box));
				let _ = document.get_element_by_id("canvas").expect(l!()).remove_attribute("style");
				let onmessage = move|e: MessageEvent| {
					let juice = juice.clone();
					let equipment = equipment.clone();
					let messages = messages.clone();
					let team = team.clone();
					let move_options = move_options.clone();
					let opponent = opponent.clone();
					let opponent_name = opponent_name.clone();
					let recording = recording.clone();
					let selected = selected.clone();
					let state = state.clone();
					let depth = depth.clone();
					let gold = gold.clone();
					use self::ServerPacket::*;
					recv(&e, move|p| {
						match p {
							Message(m) => messages.borrow_mut().push((m, MESSAGE_DURATION)),
							SafeZoneInfo(mo, mut u, mut e, j) => { move_options.replace(mo); juice.set(j);
								for u in u.iter_mut() {
									for p in u.perks.iter_mut().chain(u.perk_choice.iter_mut().flat_map(|p| p.iter_mut())) {
										let mut s = p.desc.chars().collect();
										splittify(&mut s, DESC_WIDTH);
										p.desc = s.iter().cloned().collect();
									}
									for e in u.equipment.iter_mut() {
										e.as_mut().map(|e| {
											let mut s = e.desc.chars().collect();
											splittify(&mut s, DESC_WIDTH);
											if s.last().map(|&s| s == '\n').unwrap_or(false) { s.pop(); }
											e.desc = s.iter().cloned().collect();
										});
									}
								}
								for e in e.iter_mut() {
									let mut s = e.desc.chars().collect();
									splittify(&mut s, DESC_WIDTH);
									if s.last().map(|&s| s == '\n').unwrap_or(false) { s.pop(); }
									e.desc = s.iter().cloned().collect();
								}
								state.replace(SafeZone(u, e));
							},
							Team(mut t, d, g, mut e) => {
								for u in t.iter_mut() {
									for p in u.perks.iter_mut().chain(u.perk_choice.iter_mut().flat_map(|p| p.iter_mut())) {
										let mut s = p.desc.chars().collect();
										splittify(&mut s, DESC_WIDTH);
										p.desc = s.iter().cloned().collect();
									}
									for e in u.equipment.iter_mut() {
										e.as_mut().map(|e| {
											let mut s = e.desc.chars().collect();
											splittify(&mut s, DESC_WIDTH);
											if s.last().map(|&s| s == '\n').unwrap_or(false) { s.pop(); }
											e.desc = s.iter().cloned().collect();
										});
									}
								}
								for e in e.iter_mut() {
									let mut s = e.desc.chars().collect();
									splittify(&mut s, DESC_WIDTH);
									if s.last().map(|&s| s == '\n').unwrap_or(false) { s.pop(); }
									e.desc = s.iter().cloned().collect();
								}
								team.replace(t); depth.set(d+1); gold.set(g); equipment.replace(e);
							},
							Opponent(mut o, name) => {
								for o in o.iter_mut() {
									for p in o.perks.iter_mut() {
										let mut s = p.desc.chars().collect();
										splittify(&mut s, DESC_WIDTH);
										p.desc = s.iter().cloned().collect();
									}
									for e in o.equipment.iter_mut() {
										e.as_mut().map(|e| {
											let mut s = e.desc.chars().collect();
											splittify(&mut s, DESC_WIDTH);
											if s.last().map(|&s| s == '\n').unwrap_or(false) { s.pop(); }
											e.desc = s.iter().cloned().collect();
										});
									}
								}
								opponent.replace(o); opponent_name.replace(name); state.replace(InFight(false));
							},
							FightResult(mut r, name) => {
								opponent_name.replace(name);
								if r.won {
									messages.borrow_mut().push(("won fight".into(), MESSAGE_DURATION));
								} else {
									messages.borrow_mut().push(("lost fight".into(), MESSAGE_DURATION));
								}
								for x in r.stuff.values_mut() {
									for p in x.0.iter_mut() {
										let mut s = p.desc.chars().collect();
										splittify(&mut s, DESC_WIDTH);
										p.desc = s.iter().cloned().collect();
									}
									for e in x.1.iter_mut() {
										e.as_mut().map(|e| {
											let mut s = e.desc.chars().collect();
											splittify(&mut s, DESC_WIDTH);
											if s.last().map(|&s| s == '\n').unwrap_or(false) { s.pop(); }
											e.desc = s.iter().cloned().collect();
										});
									}
								}
								state.replace(Looting);
								recording.replace(Some((r, 0, false)));
							}
							Loot => {
								state.replace(Looting);
							},
						}
						let mut s = selected.borrow_mut();
						let t = team.borrow();
						let state = state.borrow();
						if let Some((ref mut s, Some(i))) = &mut*s {
							use InventoryType::*;
							match *i {
								Team(i) => t.get(i),
								UnitStorage(i) => state.storage().and_then(|(us, _es)| us.get(i)),
								_ => unimplemented!(),
							}.and_then(|u| u.select(false).map(|s2| *s = s2));
						}
					});
				};
				onmessage(e2.clone());
				let onmessage = Closure::wrap(Box::new(onmessage) as Box<dyn FnMut(_)>);
				ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
				onmessage.forget();
				request_animation_frame(h.borrow().as_ref().expect(l!()));
			},
		});
	}) as Box<dyn FnMut(_)>);
	ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
	onmessage.forget();
	
	let onerror = Closure::wrap(Box::new(move|e: ErrorEvent| {
		log!("websocket error:"); log!("{}",e.message());
	}) as Box<dyn FnMut(_)>);
	ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
	onerror.forget();
	
	let login_result = document.get_element_by_id("login result").expect(l!());
	let name = document.get_element_by_id("name").expect(l!()).dyn_into::<HtmlInputElement>()?;
	let pswd = document.get_element_by_id("pswd").expect(l!()).dyn_into::<HtmlInputElement>()?;
	let remember_login = document.get_element_by_id("remember login").expect(l!()).dyn_into::<HtmlInputElement>()?;
	let create_new = document.get_element_by_id("create new").expect(l!()).dyn_into::<HtmlInputElement>()?;
	let ws2 = ws.clone();
	let onclick = Closure::wrap(Box::new(move|_: MouseEvent| {
		let name = name.value();
		let pswd = pswd.value();
		if !name.is_empty() && !pswd.is_empty() {
			if let Ok(name) = ArrayString::from(&name) {
				let create_new = create_new.checked();
				send_any(&ws2, &(create_new, AuthInfo { name, data: hash(&serialize(&pswd).expect(l!())), })).expect(l!());
				if remember_login.checked() {
					log!("login remembering not implemented yet sorry");
				}
			} else {
				login_result.set_inner_html("name too long");
			}
		}
	}) as Box<dyn FnMut(_)>);
	document.get_element_by_id("login button").expect(l!()).add_event_listener_with_callback("click", onclick.as_ref().unchecked_ref())?;
	onclick.forget();
	
	let mut frame_num = 0;
	*g.borrow_mut() = Some(Closure::wrap(Box::new(move|| {
		context.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);
		context.clear_depth(-1.1);
		let mut verts = Vec::new();
		let mut verts2 = Vec::new(); //because of transparency shenanigans
		{
			let v = &mut verts;
			let v2 = &mut verts2;
			let m = mouse.get();
			let d = drag_pos.get();
			let state = state.borrow();
			let mut recording = recording.borrow_mut();
			if let Some(r) = recording.as_mut() {
				pause_button.borrow().draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
				rewind_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
				skip_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
				let (t, o) = r.0.get(r.1);
				draw_team(v, v2, m, d, &t, false);
				draw_opponent(v, v2, m, d, &o);
				draw_opponent_name(v2, *opponent_name.borrow());
				frame_num += 1;
				if frame_num % FRAMES_PER_SNAPSHOT == 0 && !r.2 {
					r.1 += 1;
					if r.1 >= r.0.snapshots.len() {
						*recording = None;
					}
				}
			} else {
				match &*state {
					SafeZone(unit_storage, equipment_storage) => {
						draw_equipment(v, v2, m, d, safe_equip_box_pos(), equipment_storage);
						let t = team.borrow();
						draw_team(v, v2, m, d, &t, true);
						purchase_unit_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
						repair_button.borrow().draw(v, v2, Vec2::zero(), Vec2::one(), m, d, true);
						juice_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
						if t.iter().any(|u| u.hp + 1e-8 < u.max_hp) {
							heal_all_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
						}
						let mo = move_options.borrow();
						if !mo.is_empty() {
							let button_start = vec2(-(4.5 * 0.1 + 5.0), (mo.len() / 10) as f32 * 0.55 + 0.5) * mo[0].size(Vec2::one(), false);
							let mut p = button_start;
							let step = mo[0].size(Vec2::one(), false) * 1.1;
							for i in 0..mo.len() {
								mo[i].draw(v, v2, p, Vec2::one(), m, d, false);
								if (i+1) % 10 == 0 {
									p.y -= step.y;
									p.x = button_start.x;
								} else {
									p.x += step.x;
								}
							}
						}
						draw_unit_storage(v, v2, m, d, unit_storage);
					},
					Looting => {
						draw_depth(v2, depth.get());
						let t = team.borrow();
						draw_team(v, v2, m, d, &t, false);
						up_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
						stay_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
						down_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
					},
					InQueue => {
						draw_depth(v2, depth.get());
						let t = team.borrow();
						draw_team(v, v2, m, d, &t, false);
					},
					InFight(chosen) => {
						draw_depth(v2, depth.get());
						let t = team.borrow();
						draw_team(v, v2, m, d, &t, false);
						let o = opponent.borrow();
						draw_opponent(v, v2, m, d, &o);
						if !chosen {
							fight_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
							do_not_button.draw(v, v2, Vec2::zero(), Vec2::one(), m, d, false);
						}
						draw_opponent_name(v2, opponent_name.borrow().clone());
					},
				}
			}
			let mut messages = messages.borrow_mut();
			if !messages.is_empty() {
				if messages[0].1 <= 0.0 {
					messages.remove(0);
				}
				let mut p = vec2(right_edge(), bottom_edge());
				for m in messages.iter_mut() {
					draw_string_blended(v2, (p - vec2(TEXT_SIZE.x, 0.0) * m.0.len() as f32).extend(30.0), TEXT_SIZE, m.0.clone(), (m.1 / 2.0).min(1.0), [0.0; 4]);
					m.1 -= 1.0 / 60.0;
					p.y += TEXT_SIZE.y * 1.1;
				}
			}
			draw_string(v2, (vec2(left_edge(), top_edge()) + vec2(TEXT_SIZE.x, -TEXT_SIZE.y) * 1.1).extend(0.0), TEXT_SIZE, &format!("gold: {:.2}",gold.get()), None);
			if state.is_safe_zone() {
				let s = format!("knife juice: {:.2}",juice.get());
				draw_string(v2, (vec2(right_edge() - (s.len() as f32 + 1.1) * TEXT_SIZE.x, safe_equip_box_pos().y - TEXT_SIZE.y)).extend(0.0), TEXT_SIZE, &s, None);
			}
			selected.borrow().as_ref().map(|(s, i)| s.draw(v, v2, vec2(left_edge(), bottom_edge()), Vec2::one(), m, d, i.is_some()));
			let e = equipment.borrow();
			draw_equipment(v, v2, m, d, equip_box_pos(), &e);
		}
		verts.extend(verts2.drain(..));
		render(verts, &context);
		
		request_animation_frame(f.borrow().as_ref().expect(l!()));
	}) as Box<dyn FnMut()>));
	
	Ok(())
}

fn draw_unit_storage(v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, m: Vec2<f32>, d: Option<Vec2<f32>>, u: &Vec<Unit>) {
	quad(v, unit_storage_box_pos().extend(0.0), unit_storage_box_size(), Color(VERY_DARK_GREY));
	if !u.is_empty() {
		let size = u[0].size(storage_unit_scale(), false);
		for i in 0..u.len() {
			u[i].draw(v, v2, storage_unit_pos(i, size) + unit_storage_box_pos(), storage_unit_scale(), m, d, false);
		}
	}
}

fn unit_storage_box_pos() -> Vec2<f32> {
	vec2(left_edge() + TEXT_SIZE.x, top_edge() - TEXT_SIZE.y) - vec2(0.0, unit_storage_box_size().y)
}

fn unit_storage_box_size() -> Vec2<f32> {
	UNIT_SIZE * storage_unit_scale() * vec2(14.0, 7.0) * 1.1
}

fn storage_unit_pos(i: usize, size: Vec2<f32>) -> Vec2<f32> {
	vec2(i % 14, 6 - i / 14).f32() * size * 1.1 + size * 0.05
}

fn storage_unit_scale() -> Vec2<f32> {
	vec2(0.2, 0.2)
}

fn draw_team(v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, m: Vec2<f32>, d: Option<Vec2<f32>>, t: &Vec<Unit>, heal_buttons: bool) {
	if !t.is_empty() {
		let size = t[0].size(Vec2::one(), heal_buttons);
		for i in 0..t.len() {
			t[i].draw(v, v2, team_unit_pos(t.len(), i, size), Vec2::one(), m, d, heal_buttons);
		}
	}
}

fn team_unit_pos(len: usize, i: usize, size: Vec2<f32>) -> Vec2<f32> {
	vec2((-(len as f32) / 2.0 - 0.1 + i as f32 * 1.1) * size.x, - 0.3 - size.y)
}

fn draw_opponent<T: Thing<Args=bool>>(v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, m: Vec2<f32>, d: Option<Vec2<f32>>, o: &Vec<T>) {
	if !o.is_empty() {
		let size = o[0].size(Vec2::one(), false);
		for i in 0..o.len() {
			o[i].draw(v, v2, opponent_unit_pos(o.len(), i, size), Vec2::one(), m, d, false);
		}
	}
}

fn opponent_unit_pos(len: usize, i: usize, size: Vec2<f32>) -> Vec2<f32> {
	vec2((-(len as f32) / 2.0 - 0.1 + i as f32 * 1.1) * size.x, 0.3)
}

fn draw_equipment(v: &mut Vec<Vertex>, v2: &mut Vec<Vertex>, m: Vec2<f32>, d: Option<Vec2<f32>>, pos: Vec2<f32>, e: &Vec<Equipment>) {
	quad(v, pos.extend(0.0), equip_box_size(), Color(VERY_DARK_GREY));
	for i in 0..e.len() {
		e[i].draw(v, v2, pos + equip_pos(i), equip_scale(), m, d, true);
	}
}

fn equip_pos(i: usize) -> Vec2<f32> {
	(vec2(i % 10, 9usize.wrapping_sub(i / 10)).f32() + 0.05) * equip_box_thing_size()
}

fn equip_box_size() -> Vec2<f32> {
	equip_box_thing_size() * 10.1
}

fn safe_equip_box_pos() -> Vec2<f32> {
	vec2(right_edge() - 0.06, top_edge() - 0.06) - equip_box_size()
}

fn equip_box_pos() -> Vec2<f32> {
	vec2(right_edge() - 0.06, bottom_edge() + 0.06) - vec2(equip_box_size().x, 0.0)
}

fn equip_box_thing_size() -> Vec2<f32> {
	let x: Option<Equipment> = None;
	x.size(equip_scale(), false)
}

fn equip_scale() -> Vec2<f32> {
	vec2(0.4, 0.4)
}

fn draw_opponent_name(v2: &mut Vec<Vertex>, name: ArrayString<[u8; 32]>) {
	let size = vec2(0.1, 0.1);
	let pos: Vec2<f32> = -size * vec2(name.len(), 1).f32() * 0.5 + vec2(0.0, 0.25);
	draw_string(v2, pos.extend(20.0), size, &name, None);
}

fn draw_depth(v2: &mut Vec<Vertex>, d: usize) {
	if d > 0 {
		let s = format!("depth: {}",d);
		let size = vec2(0.05, 0.05);
		let pos: Vec2<f32> = -size * vec2(s.len(), 0).f32() * 0.5 - vec2(0.0, 1.0);
		draw_string(v2, pos.extend(20.0), size, &s, None);
	}
}
