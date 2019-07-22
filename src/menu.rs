use termion::{*, input::TermRead, event::Key};

use std::io::Write;

/// A horizontal (x by 1) list of menus. Think 'File  Edit  Selection  View ...'
pub struct MenuBar {
    pub selection_index: usize,
    pub menus: Vec<(String, Menu)>,
}

/// A vertical menu of possible actions, which one could possibly expand a sub-menu.
///
/// These are usually rendered by the MenuBar when a menu item was selected.
pub struct Menu {
    pub children: Vec<(String, MenuAction)>,
}

#[derive(Debug)]
pub enum Action {
    // Hardcoded menus //

    // File
    Close, New, Save, SaveAs, Open,

    // Help
    About,

    // A script made this action (we need to call it)
    Scripted,
}

pub enum MenuAction {
    Separator,
    Action(Action),
    SubMenu(Menu),
}

fn get_menu_shortcut_from_name(name: &str) -> char {
    let mut chars = name.chars();
    while let Some(c) = chars.next() {
        if c == '_' {
            return chars
                .next()
                .expect("Menu item name had '_' with no following shortcut letter.");
        }
    }
    panic!("Menu item had no shortcut.");
}

impl MenuBar {
    pub fn render<S: Write>(&self, s: &mut S, origin: (u16, u16), h_size: usize, focused: bool) {
        crate::util::draw_rectangle(s, &color::White, origin, (h_size, 1));
        write!(s, "{}", color::Bg(color::White)).unwrap();

        write!(s, "{}", cursor::Goto(origin.0 + 1, origin.1)).unwrap();
        for (i, (name, _)) in self.menus.iter().enumerate() {
            let is_help: bool;
            if &name[..] == "_Help" { // This is the help menu, we place it at the far right
                is_help = true;
                write!(s, "{}{}", cursor::Save, cursor::Goto(origin.0 + h_size as u16 - name.len() as u16 - 2, origin.1)).unwrap();
            } else {
                is_help = false;
            }

            write!(s, "{} {} ", if focused && i == self.selection_index { format!("{}{}", color::Bg(color::Black), color::Fg(color::White)) } else { format!("{}{}", color::Bg(color::White), color::Fg(color::Black)) },
                {
                    let mut formatted = String::new();
                    let mut chars = name.chars();
                    while let Some(c) = chars.next() {
                        if c == '_' {
                            if focused {
                                formatted.push_str(&format!(
                                    "{}{}{}",
                                    color::Fg(color::LightWhite),
                                    chars.next().unwrap(),
                                    if focused && i == self.selection_index { format!("{}", color::Fg(color::White)) } else { format!("{}", color::Fg(color::Black)) },
                                ));
                            } else {
                                formatted.push(chars.next().unwrap());
                            }
                        } else {
                            formatted.push(c);
                        }
                    }
                    formatted
                }).unwrap();
            
            if is_help {
                write!(s, "{}", cursor::Restore).unwrap(); // If we skipped to the end to print help, let's go back
            }
        }
    }

    fn get_origin_x_of_menu(&self, idx: usize) -> u16 {
        assert!(!self.menus.is_empty());
        if self.menus[idx].0 == "_Help" { // Annoying, Help is planted on the far right for style
            terminal_size().unwrap().0 - 7
        } else {
            (1 + self.menus.iter().take(idx).map(|(name, _)| name.len()).sum::<usize>() // We have a single space before menus are listed off
            + (idx + 1) * 1) // For spaces before and after names (number of items * 1)
            as u16
        }
    }

    /// Returns a menu and the origin X offset of the menu, for rendering the menu in the correct position.
    pub fn maybe_handle_key_press(&mut self, key: Key) -> Option<(&Menu, u16)> {
        match key {
            Key::Right => if self.selection_index + 1 >= self.menus.len() { self.selection_index = 0; } else { self.selection_index += 1; },
            Key::Left => if self.selection_index as isize - 1 < 0 { self.selection_index = self.menus.len()-1; } else { self.selection_index -= 1; },
            Key::Char('\n') => return Some((&self.menus[self.selection_index].1, self.get_origin_x_of_menu(self.selection_index))),
            Key::Char(key) => {
                let key = key.to_lowercase().next().unwrap();
                for (i, (c, menu)) in self
                    .menus
                    .iter()
                    .map(|(s, m)| (get_menu_shortcut_from_name(s), m))
                    .enumerate()
                {
                    if c.to_lowercase().next().unwrap() == key {
                        // Position menu's origin to directly beneath the menu bar's item
                        

                        return Some((menu, self.get_origin_x_of_menu(i)));
                    }
                }
            }
            _ => {},
        }
        None
    }
}

impl Menu {
    pub fn render<S: Write>(&self, s: &mut S, origin: (u16, u16), selection_index: usize) {
        let width = self.get_menu_width();

        // Render background box
        crate::util::draw_rectangle(s, &color::White, origin, (width, self.children.len() + 2));

        // Render box outline
        crate::util::draw_thin_unfilled_rectangle(s, &color::Black, &color::White, origin, (width, self.children.len() + 2));

        for (i, (name, a)) in self.children.iter().enumerate() {
            // goto, print name ; note the spaces before and after name (padding)
            write!(s, "{}{}{}", cursor::Goto(origin.0 + 1, origin.1 + 1 + i as u16),
                // Background of a selected item is brighter than others
                if i == selection_index { format!("{}{}", color::Bg(color::Black), color::Fg(color::White)) } else { format!("{}{}", color::Bg(color::White), color::Fg(color::Black)) },
                match a {
                    MenuAction::Separator => "─".repeat(width - 2), // width - 2 is the maximum name length
                    _ => {
                        let mut formatted = String::new();
                        let mut chars = name.chars();
                        while let Some(c) = chars.next() {
                            if c == '_' {
                                formatted.push_str(&format!(
                                    "{}{}{}",
                                    color::Fg(color::LightWhite),
                                    chars.next().unwrap(),
                                    if i == selection_index { format!("{}", color::Fg(color::White)) } else { format!("{}", color::Fg(color::Black)) }
                                ));
                            } else {
                                formatted.push(c);
                            }
                        }
                        formatted.push_str(&" ".repeat(width - 2 - if name.contains("_") { name.len() - 1 } else { name.len() } ));
                        formatted
                    }
                },
            ).unwrap();
        }
    }

    /// Take over the current thread and handle the menu's input. This causes recursion when expanding
    /// sub-menus.
    pub fn take_over<S: Write>(&self, s: &mut S, x_offset: u16) -> Option<&Action> {
        let mut selection_index = 0usize;
        loop {
            self.render(s, (x_offset, 2), selection_index);

            s.flush().unwrap();

            // All of the input code for a graphical menu.
            if let Some(k) = std::io::stdin().keys().next() {
                match k.unwrap() {
                    Key::Up => selection_index = self.previous(selection_index),
                    Key::Down => selection_index = self.next(selection_index),

                    // Activate an action or sub-menu expansion using the enter key.
                    Key::Char('\n') => match &self.children[selection_index].1 {
                        MenuAction::Separator => unreachable!(),
                        MenuAction::Action(action) => return Some(action),
                        MenuAction::SubMenu(menu) => match menu.take_over(s, x_offset + self.get_menu_width() as u16) {
                            Some(action) => return Some(action),
                            _ => {} // We don't want to close this menu if they exited out of the sub-child one.
                        },
                    },

                    // Activate an action or sub-menu expansion using a shortcut.
                    Key::Char(c) => if let Some(menu_action) = self.maybe_handle_key_press(c) {
                        match menu_action {
                            MenuAction::Separator => unreachable!(),
                            MenuAction::Action(action) => return Some(action),
                            MenuAction::SubMenu(menu) => match menu.take_over(s, x_offset + self.get_menu_width() as u16) {
                                Some(action) => return Some(action),
                                _ => {} // We don't want to close the menu... same as above ^
                            }
                        }
                    } else {
                        break; // For now, when you press an unknown key it will close the menu.
                    },

                    _ => break,
                }
            }
        }
        None
    }

    fn previous(&self, mut selection_index: usize) -> usize {
        // Perform reverse wrapping
        if selection_index as isize - 1 < 0 { selection_index = self.children.len()-1; } else { selection_index -= 1; }
        match self.children[selection_index] { // Skip separators
            (_, MenuAction::Separator) => return self.previous(selection_index),
            _ => {}
        }
        selection_index
    }

    fn next(&self, mut selection_index: usize) -> usize {
        // Perform forward selection wrapping
        if selection_index + 1 >= self.children.len() { selection_index = 0; } else { selection_index += 1; }
        match self.children[selection_index] { // Skip separators (an infinite loop in rare cases)
            (_, MenuAction::Separator) => return self.next(selection_index),
            _ => {}
        }
        selection_index
    }

    /// Returns the minimum width of the menu, without counting any underscores.
    fn get_menu_width(&self) -> usize {
        2 + self.children.iter().map(|(name, _)| if name.contains("_") { name.len() - 1 } else { name.len() }).max().expect(
            "Empty menu has no width"
        )
    }

    /// Returns `true` if the key press was correctly handled,
    /// or `false` if the key could not be handled (or was not recognized).
    fn maybe_handle_key_press(&self, key: char) -> Option<&MenuAction> {
        let key = key.to_lowercase().next().unwrap();
        for (c, menu) in self
            .children
            .iter()
            .filter_map(|(s, a)| match a { MenuAction::Separator=>None, _=>Some((get_menu_shortcut_from_name(s), a)) }) // Ignore separators, too
        {
            if c.to_lowercase().next().unwrap() == key {
                return Some(menu);
            }
        }
        None
    }
}
