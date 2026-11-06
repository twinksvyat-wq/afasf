//! Static game content: cosmetics, fighter archetypes, celebrations, opponent ladder.
use std::ffi::CStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Slot {
    Skin = 0,
    Hair = 1,
    HairColor = 2,
    Beard = 3,
    Head = 4,
    Face = 5,
    Eyes = 6,
    Neck = 7,
    Top = 8,
    Bottom = 9,
    Gloves = 10,
    Shoes = 11,
}
pub const NSLOT: usize = 12;

pub const SLOT_NAMES: [&CStr; NSLOT] = [
    c"Кожа", c"Причёска", c"Цвет волос", c"Борода", c"Головной убор", c"Маска",
    c"Очки", c"Цепь", c"Верх", c"Низ", c"Перчатки", c"Обувь",
];

#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub id: u16,
    pub slot: Slot,
    pub style: u8,
    pub name: &'static CStr,
    pub price: u32,
    pub c1: [f32; 3],
    pub c2: [f32; 3],
}

const fn it(id: u16, slot: Slot, style: u8, name: &'static CStr, price: u32, c1: [f32; 3], c2: [f32; 3]) -> Item {
    Item { id, slot, style, name, price, c1, c2 }
}
const K: [f32; 3] = [0.08, 0.08, 0.09];
const W: [f32; 3] = [0.96, 0.96, 0.96];
const GOLD: [f32; 3] = [1.0, 0.76, 0.18];
const SILVER: [f32; 3] = [0.78, 0.8, 0.84];
const RED: [f32; 3] = [0.92, 0.12, 0.14];
const BLUE: [f32; 3] = [0.12, 0.35, 0.95];

use Slot::*;
pub static ITEMS: &[Item] = &[
    // ---- Skin tones (free)
    it(1, Skin, 0, c"Светлая", 0, [0.98, 0.80, 0.68], K),
    it(2, Skin, 0, c"Персиковая", 0, [0.93, 0.70, 0.55], K),
    it(3, Skin, 0, c"Оливковая", 0, [0.80, 0.58, 0.40], K),
    it(4, Skin, 0, c"Смуглая", 0, [0.62, 0.42, 0.28], K),
    it(5, Skin, 0, c"Тёмная", 0, [0.42, 0.27, 0.18], K),
    it(6, Skin, 0, c"Эбеновая", 0, [0.27, 0.17, 0.12], K),
    it(7, Skin, 0, c"Зомби", 900, [0.55, 0.75, 0.50], K),
    it(8, Skin, 0, c"Аватар", 1200, [0.35, 0.60, 0.95], K),
    // ---- Hair styles
    it(20, Hair, 0, c"Лысый", 0, K, K),
    it(21, Hair, 1, c"Ёжик", 0, K, K),
    it(22, Hair, 2, c"Ирокез", 250, K, K),
    it(23, Hair, 3, c"Афро", 300, K, K),
    it(24, Hair, 4, c"Пучок", 200, K, K),
    it(25, Hair, 5, c"Шипы", 350, K, K),
    // ---- Hair colors
    it(30, HairColor, 0, c"Чёрный", 0, [0.07, 0.06, 0.06], K),
    it(31, HairColor, 0, c"Каштан", 0, [0.35, 0.20, 0.10], K),
    it(32, HairColor, 0, c"Блонд", 0, [0.95, 0.80, 0.45], K),
    it(33, HairColor, 0, c"Рыжий", 100, [0.85, 0.35, 0.10], K),
    it(34, HairColor, 0, c"Седой", 100, [0.88, 0.88, 0.88], K),
    it(35, HairColor, 0, c"Электрик", 250, [0.15, 0.45, 1.0], K),
    it(36, HairColor, 0, c"Розовый", 250, [1.0, 0.35, 0.70], K),
    it(37, HairColor, 0, c"Токсик", 250, [0.45, 1.0, 0.20], K),
    // ---- Beards
    it(40, Beard, 0, c"Без бороды", 0, K, K),
    it(41, Beard, 1, c"Щетина", 0, K, K),
    it(42, Beard, 2, c"Борода", 150, K, K),
    it(43, Beard, 3, c"Эспаньолка", 150, K, K),
    // ---- Head
    it(50, Head, 0, c"Ничего", 0, K, K),
    it(51, Head, 1, c"Кепка", 200, RED, W),
    it(52, Head, 2, c"Шапка", 200, [0.15, 0.15, 0.18], [0.9, 0.3, 0.2]),
    it(53, Head, 3, c"Повязка", 120, RED, RED),
    it(54, Head, 4, c"Корона", 2500, GOLD, RED),
    it(55, Head, 5, c"Ковбойская", 600, [0.55, 0.35, 0.18], [0.2, 0.12, 0.06]),
    it(56, Head, 1, c"Кепка «Ночь»", 300, K, GOLD),
    // ---- Face
    it(60, Face, 0, c"Ничего", 0, K, K),
    it(61, Face, 1, c"Лучадор", 800, [0.1, 0.4, 1.0], GOLD),
    it(62, Face, 2, c"Балаклава", 400, K, K),
    it(63, Face, 3, c"Бандана", 250, RED, W),
    it(64, Face, 4, c"Клоунский нос", 150, [1.0, 0.1, 0.1], K),
    it(65, Face, 1, c"Лучадор «Кровь»", 900, RED, K),
    // ---- Eyes
    it(70, Eyes, 0, c"Ничего", 0, K, K),
    it(71, Eyes, 1, c"Тёмные очки", 200, K, K),
    it(72, Eyes, 2, c"Авиаторы", 450, [0.25, 0.15, 0.05], GOLD),
    it(73, Eyes, 3, c"Горнолыжные", 500, [1.0, 0.55, 0.0], K),
    it(74, Eyes, 4, c"Сердечки", 350, [1.0, 0.2, 0.5], [1.0, 0.2, 0.5]),
    // ---- Neck
    it(80, Neck, 0, c"Ничего", 0, K, K),
    it(81, Neck, 1, c"Золотая цепь", 1000, GOLD, GOLD),
    it(82, Neck, 1, c"Серебряная цепь", 500, SILVER, SILVER),
    it(83, Neck, 2, c"Бусы", 200, [0.9, 0.2, 0.3], [0.2, 0.7, 0.9]),
    it(84, Neck, 3, c"Бабочка", 300, RED, RED),
    // ---- Tops
    it(90, Top, 0, c"Голый торс", 0, K, K),
    it(91, Top, 1, c"Майка", 0, W, W),
    it(92, Top, 2, c"Футболка", 150, [0.1, 0.1, 0.12], RED),
    it(93, Top, 3, c"Худи", 400, [0.55, 0.58, 0.62], [0.3, 0.3, 0.34]),
    it(94, Top, 4, c"Рашгард", 350, BLUE, K),
    it(95, Top, 2, c"Футболка «Огонь»", 250, [1.0, 0.45, 0.05], [1.0, 0.85, 0.1]),
    it(96, Top, 1, c"Майка «Алая»", 100, RED, RED),
    // ---- Bottoms
    it(100, Bottom, 0, c"Шорты ММА", 0, [0.1, 0.1, 0.12], RED),
    it(101, Bottom, 0, c"Шорты красные", 0, RED, W),
    it(102, Bottom, 1, c"Боксёрские трусы", 200, GOLD, K),
    it(103, Bottom, 2, c"Спортивки", 250, [0.2, 0.22, 0.26], W),
    it(104, Bottom, 2, c"Штаны ги", 300, W, K),
    it(105, Bottom, 0, c"Шорты синие", 100, BLUE, W),
    it(106, Bottom, 1, c"Трусы «Чемпион»", 600, [0.5, 0.0, 0.8], GOLD),
    // ---- Gloves
    it(110, Gloves, 0, c"ММА чёрные", 0, K, K),
    it(111, Gloves, 0, c"ММА красные", 0, RED, K),
    it(112, Gloves, 1, c"Боксёрские", 200, RED, W),
    it(113, Gloves, 1, c"Боксёрские синие", 200, BLUE, W),
    it(114, Gloves, 2, c"Бинты", 100, W, W),
    it(115, Gloves, 1, c"Золотые", 1500, GOLD, W),
    it(116, Gloves, 0, c"ММА неон", 300, [0.4, 1.0, 0.2], K),
    // ---- Shoes
    it(120, Shoes, 0, c"Босиком", 0, K, K),
    it(121, Shoes, 1, c"Кроссовки", 150, W, RED),
    it(122, Shoes, 2, c"Боксёрки", 250, K, W),
    it(123, Shoes, 3, c"Шлёпки", 50, BLUE, W),
    it(124, Shoes, 1, c"Золотые кроссы", 1200, GOLD, K),
    it(125, Shoes, 2, c"Боксёрки красные", 300, RED, W),
];

pub fn item_by_id(id: u16) -> Option<&'static Item> {
    ITEMS.iter().find(|i| i.id == id)
}

/// Equipped item per slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub items: [u16; NSLOT],
}
impl Look {
    pub fn default_player() -> Look {
        Look { items: [2, 21, 30, 41, 50, 60, 70, 80, 90, 100, 111, 120] }
    }
    pub fn get(&self, s: Slot) -> &'static Item {
        item_by_id(self.items[s as usize]).unwrap_or(&ITEMS[0])
    }
    pub fn set(&mut self, id: u16) {
        if let Some(i) = item_by_id(id) {
            self.items[i.slot as usize] = id;
        }
    }
}

// ---------------------------------------------------------------- fighters
#[derive(Clone, Copy, Debug)]
pub struct Archetype {
    pub id: u32,
    pub name: &'static CStr,
    pub style: &'static CStr,
    pub price: u32,
    pub height: f32,
    pub bulk: f32,
    pub power: f32,
    pub speed: f32,
    pub kick: f32,
    pub grapple: f32,
    pub chin: f32,
    pub finisher: u8, // 0 superman punch, 1 spinning back kick, 2 haymaker, 3 suplex, 4 flying knee
}

pub static ARCHETYPES: &[Archetype] = &[
    Archetype { id: 0, name: c"Рокки", style: c"Боксёр · сбалансированный", price: 0, height: 1.0, bulk: 1.0, power: 1.0, speed: 1.0, kick: 0.85, grapple: 0.9, chin: 1.0, finisher: 0 },
    Archetype { id: 1, name: c"Тайфун", style: c"Муай-тай · ноги и колени", price: 600, height: 1.04, bulk: 0.9, power: 0.95, speed: 1.05, kick: 1.3, grapple: 0.9, chin: 0.95, finisher: 1 },
    Archetype { id: 2, name: c"Медведь", style: c"Тяжеловес · медленный, мощный", price: 900, height: 1.07, bulk: 1.3, power: 1.35, speed: 0.82, kick: 0.9, grapple: 1.1, chin: 1.3, finisher: 2 },
    Archetype { id: 3, name: c"Змей", style: c"Борец · захваты и броски", price: 1200, height: 0.98, bulk: 1.1, power: 0.95, speed: 0.95, kick: 0.9, grapple: 1.5, chin: 1.1, finisher: 3 },
    Archetype { id: 4, name: c"Призрак", style: c"Спидстер · быстрый, хрупкий", price: 1500, height: 0.95, bulk: 0.85, power: 0.9, speed: 1.3, kick: 1.1, grapple: 0.85, chin: 0.8, finisher: 4 },
];

pub static FINISHER_NAMES: [&CStr; 5] = [c"Супермен-панч", c"Вертушка", c"Хеймейкер", c"Суплекс", c"Летящее колено"];

// ---------------------------------------------------------------- celebrations
#[derive(Clone, Copy, Debug)]
pub struct Celebration {
    pub id: u32,
    pub name: &'static CStr,
    pub price: u32,
}
pub static CELEBRATIONS: &[Celebration] = &[
    Celebration { id: 0, name: c"Флекс", price: 0 },
    Celebration { id: 1, name: c"Сальто назад", price: 400 },
    Celebration { id: 2, name: c"Шаффл", price: 500 },
    Celebration { id: 3, name: c"Робот", price: 600 },
    Celebration { id: 4, name: c"Вертолёт", price: 700 },
    Celebration { id: 5, name: c"Горилла", price: 800 },
    Celebration { id: 6, name: c"Лунная походка", price: 900 },
    Celebration { id: 7, name: c"Флосс", price: 1000 },
];

// ---------------------------------------------------------------- opponents
#[derive(Clone, Copy, Debug)]
pub struct Opponent {
    pub name: &'static CStr,
    pub nick: &'static CStr,
    pub archetype: u32,
    pub skill: f32, // 0..1
    pub reward: u32,
    pub look: [u16; NSLOT],
}
pub static OPPONENTS: &[Opponent] = &[
    Opponent { name: c"Гоша", nick: c"Новичок с района", archetype: 0, skill: 0.10, reward: 120, look: [1, 20, 30, 40, 51, 60, 70, 80, 91, 103, 114, 121] },
    Opponent { name: c"Толик", nick: c"Дворовый задира", archetype: 2, skill: 0.22, reward: 160, look: [3, 21, 31, 42, 52, 60, 70, 82, 92, 103, 110, 123] },
    Opponent { name: c"Сан Саныч", nick: c"Старая школа", archetype: 0, skill: 0.35, reward: 200, look: [2, 20, 34, 42, 53, 60, 70, 80, 90, 102, 112, 122] },
    Opponent { name: c"Кобра", nick: c"Тайский бокс", archetype: 1, skill: 0.48, reward: 240, look: [4, 22, 30, 40, 53, 60, 71, 83, 90, 105, 114, 120] },
    Opponent { name: c"Эль Дьябло", nick: c"Король ринга", archetype: 3, skill: 0.58, reward: 280, look: [3, 20, 30, 40, 50, 65, 70, 81, 90, 106, 111, 125] },
    Opponent { name: c"Молния", nick: c"Быстрее пули", archetype: 4, skill: 0.68, reward: 320, look: [5, 25, 35, 40, 50, 60, 73, 80, 94, 101, 116, 121] },
    Opponent { name: c"Гризли", nick: c"Сибирский танк", archetype: 2, skill: 0.80, reward: 400, look: [1, 23, 33, 42, 55, 60, 72, 81, 93, 104, 113, 122] },
    Opponent { name: c"Король", nick: c"Действующий чемпион", archetype: 0, skill: 0.92, reward: 600, look: [6, 21, 30, 43, 54, 60, 71, 81, 90, 106, 115, 124] },
];
