use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MouseEvent,WebSocket,ErrorEvent,HtmlInputElement,MessageEvent};
use std::cell::Cell;

macro_rules! log {
	( $( $t:tt )* ) => {
		web_sys::console::log_1(&format!( $( $t )* ).into());
	}
}

mod vertex;
mod prelude;
use prelude::*;
mod boiler_plate;
use boiler_plate::*;
mod thing;
use thing::*;
mod collision;

const DEFAULT_IP: &str = "127.0.0.1";

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	#[cfg(feature = "console_error_panic_hook")]
	console_error_panic_hook::set_once();
	
	let document = window().document().unwrap();
	let canvas = document.get_element_by_id("canvas").unwrap();
	let canvas = Rc::new(canvas.dyn_into::<web_sys::HtmlCanvasElement>()?);
	let context = setup_rendering(&canvas, &document)?;
	let ip = window().location().href().map_err(|_| ()).and_then(|s| {
		let mut s = s.split('?'); s.next();
		s.next().and_then(|s| s.find("ip=").map(|i| {
			let s = s.split_at(i+3).1;
			s.split('&').next().unwrap().to_string()
		})).ok_or(())
	}).unwrap_or(DEFAULT_IP.to_string());
	let ws = WebSocket::new(&format!("ws://{}:2794",ip))?;
	
	let move_options: Rc<RefCell<Vec<MoveOption>>> = Rc::new(RefCell::new(Vec::new()));
	let units: Rc<RefCell<Vec<Unit>>> = Rc::new(RefCell::new(Vec::new()));
	let opponent = Rc::new(RefCell::new(Vec::new()));
	let opponent_name = Rc::new(RefCell::new(String::new()));
	let needs_fight_choice = Rc::new(Cell::new(false));
	let fight_button = Rc::new(Button {
		name: "fight".to_string(),
		pos: vec2(-0.5, 0.0) - BUTTON_SIZE * vec2(2.5, 1.5) * 0.5,
		size: BUTTON_SIZE * vec2(2.5, 1.5),
		tex: Color(RED),
	});
	let do_not_button = Rc::new(Button {
		name: "do not".to_string(),
		pos: vec2(0.5, 0.0) - BUTTON_SIZE * vec2(2.5, 1.5) * 0.5,
		size: BUTTON_SIZE * vec2(2.5, 1.5),
		tex: Color(GREEN),
	});
	
	let canvas2 = canvas.clone();
	let fight_button2 = fight_button.clone();
	let do_not_button2 = do_not_button.clone();
	let move_options2 = move_options.clone();
	let ws2 = ws.clone();
	let needs_fight_choice2 = needs_fight_choice.clone();
	let onclick = Closure::wrap(Box::new(move|e: MouseEvent| {
		let m = screen_coords(e.client_x(), e.client_y(), &canvas2);
		let mut o = move_options2.borrow_mut();
		if needs_fight_choice2.get() {
			if fight_button2.collides(m, Vec2::zero()) {
				send(&ws2, ClientPacket::Fight(true)).expect(l!());
				needs_fight_choice2.set(false);
			} else if do_not_button2.collides(m, Vec2::zero()) {
				send(&ws2, ClientPacket::Fight(false)).expect(l!());
				needs_fight_choice2.set(false);
			}
		}
		if !o.is_empty() {
			let button_start = vec2(-(4.5 * 0.1 + 5.0), (o.len() / 10) as f32 * 0.55 + 0.5) * o[0].size();
			let mut p = button_start;
			let step = o[0].size() * 1.1;
			for i in 0..o.len() {
				if o[i].collides(m, p) {
					send(&ws2, ClientPacket::Move(o[i].id)).expect(l!());
					*o = Vec::new();
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
	}) as Box<dyn Fn(MouseEvent)>);
	canvas.set_onclick(Some(onclick.as_ref().unchecked_ref()));
	onclick.forget();
	
	let drag_pos = Rc::new(Cell::new(None));
	let a = drag_pos.clone();
	let b = a.clone();
	let c = a.clone();
	let canvas2 = canvas.clone();
	let onmousedown = Closure::wrap(Box::new(move|e: MouseEvent| if e.button() == 0 { a.set(Some(screen_coords(e.client_x(), e.client_y(), &canvas2))) })
		as Box<dyn Fn(MouseEvent)>);
	canvas.set_onmousedown(Some(onmousedown.as_ref().unchecked_ref()));
	onmousedown.forget();
	
	let units2 = units.clone();
	let canvas2 = canvas.clone();
	let ws2 = ws.clone();
	let onmouseup = Closure::wrap(Box::new(move|e: MouseEvent| if e.button() == 0 {
		if let Some(initial_pos) = b.replace(None) {
			let mut to = 0;
			let mut from = 0;
			let units = units2.borrow();
			let mut p = vec2((-(units.len() as f32 / 2.0) - 0.1 * 2.5) * UNIT_SIZE.x, -0.1 - UNIT_SIZE.y);
			let m = screen_coords(e.client_x(), e.client_y(), &canvas2);
			for (i, u) in units.iter().enumerate() {
				if u.collides(initial_pos, p) {
					from = i;
					let x = (m.x - p.x) / (u.size().x * 1.1);
					to = (from as isize + x.floor() as isize + 1).max(0) as usize;
					to = to.min(units.len());
					break;
				}
				p += vec2(u.size().x * 1.1, 0.0);
			}
			if from + to != 0 {
				send(&ws2, ClientPacket::Rearrange(from, to)).expect(l!());
			}
		}
	}) as Box<dyn Fn(MouseEvent)>);
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
	}) as Box<dyn Fn(MouseEvent)>);
	canvas.set_onmousemove(Some(onmove.as_ref().unchecked_ref()));
	onmove.forget();
	
	let f = Rc::new(RefCell::new(None));
	let g = f.clone();
	let h = g.clone();
	
	let units2 = units.clone();
	let move_options2 = move_options.clone();
	let ws2 = ws.clone();
	let opponent2 = opponent.clone();
	let opponent_name2 = opponent_name.clone();
	let needs_fight_choice2 = needs_fight_choice.clone();
	let onmessage = Closure::wrap(Box::new(move|e: MessageEvent| {
		let opponent = opponent2.clone();
		let opponent_name = opponent_name2.clone();
		let needs_fight_choice = needs_fight_choice2.clone();
		let document = window().document().unwrap();
		let units = units2.clone();
		let move_options = move_options2.clone();
		use self::ServerPacket::*;
		let ws = ws2.clone();
		let h = h.clone();
		let e2 = e.clone();
		recv(&e, move|p| match p {
			Message(m) => {
				document.get_element_by_id("login result").map(|r| r.set_inner_html(&m));
			},
			_ => {
				let _ = document.get_element_by_id("login box").map(|login_box| login_box.parent_node().unwrap().remove_child(&login_box));
				document.get_element_by_id("canvas").unwrap().remove_attribute("style").expect("failed to make canvas visible");
				let units = units.clone();
				let move_options = move_options.clone();
				let opponent = opponent.clone();
				let opponent_name = opponent_name.clone();
				let needs_fight_choice = needs_fight_choice.clone();
				let onmessage = move|e: MessageEvent| {
					let units = units.clone();
					let move_options = move_options.clone();
					let opponent = opponent.clone();
					let opponent_name = opponent_name.clone();
					let needs_fight_choice = needs_fight_choice.clone();
					recv(&e, move|p| match p {
						Team(u) => { units.replace(u); },
						Opponent(n, name, view) => { needs_fight_choice.set(n); opponent_name.replace(name); opponent.replace(view); },
						MoveOptions(o) => { move_options.replace(o); },
						Message(m) => log!("{}",m),
						Fight(recording) => {
							if recording.won {
								log!("won fight");
							} else {
								log!("lost fight");
							}
							opponent.replace(Vec::new());
						},
					});
				};
				onmessage(e2.clone());
				let onmessage = Closure::wrap(Box::new(onmessage) as Box<dyn FnMut(_)>);
				ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
				onmessage.forget();
				request_animation_frame(h.borrow().as_ref().unwrap());
			},
		});
	}) as Box<dyn FnMut(_)>);
	ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
	onmessage.forget();
	
	let onerror = Closure::wrap(Box::new(move|e| {
		log!("error event: {:?}",e);
	}) as Box<dyn FnMut(ErrorEvent)>);
	ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
	onerror.forget();
	
	let login_result = document.get_element_by_id("login result").unwrap();
	let name = document.get_element_by_id("name").unwrap().dyn_into::<HtmlInputElement>()?;
	let pswd = document.get_element_by_id("pswd").unwrap().dyn_into::<HtmlInputElement>()?;
	let remember_login = document.get_element_by_id("remember login").unwrap().dyn_into::<HtmlInputElement>()?;
	let create_new = document.get_element_by_id("create new").unwrap().dyn_into::<HtmlInputElement>()?;
	let ws2 = ws.clone();
	let onclick = Closure::wrap(Box::new(move|_: MouseEvent| {
		let name = name.value();
		let pswd = pswd.value();
		if !name.is_empty() && !pswd.is_empty() {
			if let Ok(name) = serialize_small_string(&name) {
				let create_new = create_new.checked();
				send_any(&ws2, &(create_new, AuthInfo { id: name, data: hash(&serialize(&pswd).unwrap()), })).unwrap();
				if remember_login.checked() {
					log!("login remembering not implemented yet sorry");
				}
			} else {
				login_result.set_inner_html("name to long or otherwise unable to serialize");
			}
		}
	}) as Box<dyn FnMut(_)>);
	document.get_element_by_id("login button").unwrap().add_event_listener_with_callback("click", onclick.as_ref().unchecked_ref())?;
	onclick.forget();
	
	*g.borrow_mut() = Some(Closure::wrap(Box::new(move|| {
		context.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);
		context.clear_depth(-1.1);
		let mut verts = Vec::new();
		let mut verts2 = Vec::new(); //because of transparency shenanigans
		{
			let v = &mut verts;
			let v2 = &mut verts2;
			let m = mouse.get();
			let units = units.borrow();
			let mut p = vec2((-(units.len() as f32 / 2.0) - 0.1 * 2.5) * UNIT_SIZE.x, -0.2 - UNIT_SIZE.y);
			for u in units.iter() {
				u.draw(v, v2, p, m, drag_pos.get());
				p += vec2(u.size().x * 1.1, 0.0);
			}
			let opponent = opponent.borrow();
			if !opponent.is_empty() {
				let mut p = vec2((-(opponent.len() as f32 / 2.0) - 0.1 * 2.5) * UNIT_SIZE.x, 0.2);
				for u in opponent.iter() {
					u.draw(v, v2, p, m, drag_pos.get());
					p += vec2(u.size().x * 1.1, 0.0);
				}
				if needs_fight_choice.get() {
					fight_button.draw(v, v2, Vec2::zero(), m, drag_pos.get());
					do_not_button.draw(v, v2, Vec2::zero(), m, drag_pos.get());
				}
				let name = opponent_name.borrow().clone();
				let size = vec2(0.1, 0.1);
				let pos: Vec2<f32> = -size * vec2(name.len(), 1).f32() * 0.5 + vec2(0.0, 0.15);
				vertex::draw_string(v, pos.extend(20.0), size, name);
			}
			let move_options = move_options.borrow();
			if !move_options.is_empty() {
				let button_start = vec2(-(4.5 * 0.1 + 5.0), (move_options.len() / 10) as f32 * 0.55 + 0.5) * move_options[0].size();
				let mut p = button_start;
				let step = move_options[0].size() * 1.1;
				for i in 0..move_options.len() {
					move_options[i].draw(v, v2, p, m, drag_pos.get());
					if (i+1) % 10 == 0 {
						p.y -= step.y;
						p.x = button_start.x;
					} else {
						p.x += step.x;
					}
				}
			}
		}
		verts.extend(verts2.drain(..));
		render(verts, &context);
		
		request_animation_frame(f.borrow().as_ref().unwrap());
	}) as Box<dyn FnMut()>));
	
	Ok(())
}
