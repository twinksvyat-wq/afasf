//! Player progression: coins, owned items, unlocks, stats. Simple text save format.
use crate::catalog::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub fights: u32,
    pub wins: u32,
    pub losses: u32,
    pub kos: u32,
    pub finishers: u32,
    pub knockdowns: u32,
    pub kod_by: u32,
    pub punches: u32,
    pub kicks: u32,
    pub throws: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub damage: u32,
    pub coins_total: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub coins: u32,
    pub owned: Vec<u16>,
    pub look: Look,
    pub fighters: Vec<u32>,
    pub fighter: u32,
    pub celebrations: Vec<u32>,
    pub celebration: u32,
    pub ladder: u32, // highest unlocked opponent index
    pub stats: Stats,
}

impl Default for Profile {
    fn default() -> Self {
        let owned: Vec<u16> = ITEMS.iter().filter(|i| i.price == 0).map(|i| i.id).collect();
        Profile { coins: 300, owned, look: Look::default_player(), fighters: vec![0], fighter: 0, celebrations: vec![0], celebration: 0, ladder: 0, stats: Stats::default() }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Reward {
    pub coins: u32,
    pub unlocked_opponent: bool,
}

impl Profile {
    pub fn owns(&self, id: u16) -> bool {
        self.owned.contains(&id)
    }
    pub fn buy(&mut self, id: u16) -> bool {
        let Some(it) = item_by_id(id) else { return false };
        if self.owns(id) {
            return true;
        }
        if self.coins < it.price {
            return false;
        }
        self.coins -= it.price;
        self.owned.push(id);
        true
    }
    pub fn equip(&mut self, id: u16) -> bool {
        if !self.owns(id) {
            return false;
        }
        self.look.set(id);
        true
    }
    pub fn buy_fighter(&mut self, idx: u32) -> bool {
        let Some(a) = ARCHETYPES.get(idx as usize) else { return false };
        if self.fighters.contains(&idx) {
            return true;
        }
        if self.coins < a.price {
            return false;
        }
        self.coins -= a.price;
        self.fighters.push(idx);
        true
    }
    pub fn buy_celebration(&mut self, idx: u32) -> bool {
        let Some(c) = CELEBRATIONS.get(idx as usize) else { return false };
        if self.celebrations.contains(&idx) {
            return true;
        }
        if self.coins < c.price {
            return false;
        }
        self.coins -= c.price;
        self.celebrations.push(idx);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_fight(&mut self, opp: usize, won: bool, method: u32, landed: u32, punches: u32, kicks: u32, throws: u32, knockdowns: u32, damage: f32, finisher: bool) -> Reward {
        let s = &mut self.stats;
        s.fights += 1;
        s.punches += punches;
        s.kicks += kicks;
        s.throws += throws;
        s.knockdowns += knockdowns;
        s.damage += damage as u32;
        let mut coins;
        let mut unlocked = false;
        if won {
            s.wins += 1;
            s.streak += 1;
            s.best_streak = s.best_streak.max(s.streak);
            if method == 1 || method == 2 || method == 4 {
                s.kos += 1;
            }
            if finisher {
                s.finishers += 1;
            }
            coins = OPPONENTS.get(opp).map(|o| o.reward).unwrap_or(150);
            if method == 1 || method == 2 {
                coins += 80;
            }
            if finisher {
                coins += 120;
            }
            if opp as u32 >= self.ladder && (opp + 1) < OPPONENTS.len() {
                self.ladder = opp as u32 + 1;
                unlocked = true;
            }
        } else {
            s.losses += 1;
            s.streak = 0;
            if method != 3 {
                s.kod_by += 1;
            }
            coins = 40;
        }
        coins += landed.min(80);
        self.coins += coins;
        self.stats.coins_total += coins;
        Reward { coins, unlocked_opponent: unlocked }
    }

    pub fn serialize(&self) -> String {
        let s = &self.stats;
        let list = |v: &[u32]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        let owned: Vec<u32> = self.owned.iter().map(|&x| x as u32).collect();
        let look: Vec<u32> = self.look.items.iter().map(|&x| x as u32).collect();
        format!(
            "v=1\ncoins={}\nowned={}\nlook={}\nfighters={}\nfighter={}\ncels={}\ncel={}\nladder={}\nstats={}\n",
            self.coins,
            list(&owned),
            list(&look),
            list(&self.fighters),
            self.fighter,
            list(&self.celebrations),
            self.celebration,
            self.ladder,
            list(&[s.fights, s.wins, s.losses, s.kos, s.finishers, s.knockdowns, s.kod_by, s.punches, s.kicks, s.throws, s.streak, s.best_streak, s.damage, s.coins_total])
        )
    }

    pub fn deserialize(text: &str) -> Option<Profile> {
        let mut p = Profile::default();
        let nums = |v: &str| -> Vec<u32> { v.split(',').filter_map(|x| x.trim().parse().ok()).collect() };
        let mut ok = false;
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match k.trim() {
                "v" => ok = true,
                "coins" => p.coins = v.trim().parse().ok()?,
                "owned" => {
                    for id in nums(v) {
                        if item_by_id(id as u16).is_some() && !p.owned.contains(&(id as u16)) {
                            p.owned.push(id as u16);
                        }
                    }
                }
                "look" => {
                    for id in nums(v) {
                        if p.owned.contains(&(id as u16)) {
                            p.look.set(id as u16);
                        }
                    }
                }
                "fighters" => p.fighters = nums(v).into_iter().filter(|&i| (i as usize) < ARCHETYPES.len()).collect(),
                "fighter" => p.fighter = v.trim().parse().unwrap_or(0),
                "cels" => p.celebrations = nums(v).into_iter().filter(|&i| (i as usize) < CELEBRATIONS.len()).collect(),
                "cel" => p.celebration = v.trim().parse().unwrap_or(0),
                "ladder" => p.ladder = v.trim().parse::<u32>().unwrap_or(0).min(OPPONENTS.len() as u32 - 1),
                "stats" => {
                    let n = nums(v);
                    let g = |i: usize| n.get(i).copied().unwrap_or(0);
                    p.stats = Stats { fights: g(0), wins: g(1), losses: g(2), kos: g(3), finishers: g(4), knockdowns: g(5), kod_by: g(6), punches: g(7), kicks: g(8), throws: g(9), streak: g(10), best_streak: g(11), damage: g(12), coins_total: g(13) };
                }
                _ => {}
            }
        }
        if !ok {
            return None;
        }
        if !p.fighters.contains(&0) {
            p.fighters.insert(0, 0);
        }
        if !p.fighters.contains(&p.fighter) {
            p.fighter = 0;
        }
        if !p.celebrations.contains(&0) {
            p.celebrations.insert(0, 0);
        }
        if !p.celebrations.contains(&p.celebration) {
            p.celebration = 0;
        }
        Some(p)
    }
}
