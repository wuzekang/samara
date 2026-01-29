use crate::types::{Link, LinkKey, NodeKey, ReactiveFlags};

impl super::ReactiveSystem {
    /// Notify effects that need to run
    pub fn notify(&mut self, effect: NodeKey) {
        let mut effect = effect;
        let mut insert_index = self.queued_length;
        let mut first_inserted_index = insert_index;

        loop {
            if insert_index >= self.queued.len() {
                self.queued.push(effect);
            } else {
                self.queued[insert_index] = effect;
            }
            insert_index += 1;
            let subs = self.nodes[effect].subs;
            let Some(subs) = subs else {
                break;
            };
            effect = self.links[subs].sub;
            if !(self.nodes[effect].flags.contains(ReactiveFlags::WATCHING)) {
                break;
            }
        }

        self.queued_length = insert_index;
        while first_inserted_index < {
            insert_index -= 1;
            insert_index
        } {
            self.queued.swap(first_inserted_index, insert_index);
            first_inserted_index += 1;
        }
    }

    /// Handle node that is no longer watched
    pub fn unwatched(&mut self, node: NodeKey) {
        if !(self.nodes[node].flags.contains(ReactiveFlags::MUTABLE)) {
            self.purge_scope(node);
        } else if self.nodes[node].deps_tail.is_some() {
            self.nodes[node].deps_tail = None;
            self.nodes[node].flags = ReactiveFlags::MUTABLE | ReactiveFlags::DIRTY;
            self.purge_deps(node, false);
        }
    }

    /// Propagate changes through subscribers
    pub fn propagate(&mut self, link: LinkKey) {
        let mut link = link;
        let mut next = self.links[link].next_sub;
        self.stack.clear();
        'top: loop {
            let sub_key = self.links[link].sub;
            let sub = &mut self.nodes[sub_key];
            let mut flags = sub.flags;

            if !(flags.intersects(
                ReactiveFlags::RECURSED_CHECK
                    | ReactiveFlags::RECURSED
                    | ReactiveFlags::DIRTY
                    | ReactiveFlags::PENDING,
            )) {
                sub.flags = flags | ReactiveFlags::PENDING;
            } else if !(flags.contains(ReactiveFlags::RECURSED_CHECK | ReactiveFlags::RECURSED)) {
                flags = ReactiveFlags::NONE;
            } else if !(flags.contains(ReactiveFlags::RECURSED_CHECK)) {
                sub.flags = (flags & (!ReactiveFlags::RECURSED)) | ReactiveFlags::PENDING;
            } else if !(flags.contains(ReactiveFlags::DIRTY | ReactiveFlags::PENDING))
                && self.is_valid_link(link, sub_key)
            {
                self.nodes[sub_key].flags =
                    flags | (ReactiveFlags::RECURSED | ReactiveFlags::PENDING);
                flags &= ReactiveFlags::MUTABLE;
            } else {
                flags = ReactiveFlags::NONE;
            }

            if flags.contains(ReactiveFlags::WATCHING) {
                self.notify(sub_key);
            }

            if flags.contains(ReactiveFlags::MUTABLE) {
                let subs = self.nodes[sub_key].subs;
                if let Some(subs) = subs {
                    let next_sub = self.links[subs].next_sub;
                    link = subs;
                    if let Some(next_sub_val) = next_sub {
                        if let Some(next_val) = next {
                            self.stack.push(next_val);
                        }
                        next = Some(next_sub_val);
                    }
                    continue 'top;
                }
            }

            if let Some(next_sub) = next {
                link = next_sub;
                next = self.links[link].next_sub;
                continue 'top;
            }

            while let Some(l) = self.stack.pop() {
                link = l;
                next = self.links[link].next_sub;
                continue 'top;
            }

            break;
        }
    }

    /// Check if a node is dirty and needs updating
    pub fn check_dirty(&mut self, mut link: LinkKey, mut sub: NodeKey) -> bool {
        let mut check_depth = 0;
        let mut dirty = false;
        self.stack.clear();
        'top: loop {
            let dep = self.links[link].dep;
            let flags = self.nodes[dep].flags;

            if self.nodes[sub].flags.contains(ReactiveFlags::DIRTY) {
                dirty = true;
            } else if flags.contains(ReactiveFlags::MUTABLE | ReactiveFlags::DIRTY) {
                if self.update(dep) {
                    let subs = self.nodes[dep].subs.unwrap();
                    if self.links[subs].next_sub.is_some() {
                        self.shallow_propagate(subs);
                    }
                    dirty = true;
                }
            } else if flags.contains(ReactiveFlags::MUTABLE | ReactiveFlags::PENDING) {
                if self.links[link].next_sub.is_some() || self.links[link].prev_sub.is_some() {
                    self.stack.push(link);
                }
                link = self.nodes[dep].deps.unwrap();
                sub = dep;
                check_depth += 1;
                continue 'top;
            }

            if !dirty {
                if let Some(next_dep) = self.links[link].next_dep {
                    link = next_dep;
                    continue 'top;
                }
            }

            while check_depth > 0 {
                check_depth -= 1;
                let first_sub = self.nodes[sub].subs.unwrap();
                let has_multiple_subs = self.links[first_sub].next_sub.is_some();

                if has_multiple_subs {
                    link = self.stack.pop().unwrap();
                } else {
                    link = first_sub;
                }

                if dirty {
                    if self.update(sub) {
                        if has_multiple_subs {
                            self.shallow_propagate(first_sub);
                        }
                        sub = self.links[link].sub;
                        continue;
                    }
                    dirty = false;
                } else {
                    self.nodes[sub].flags.remove(ReactiveFlags::PENDING);
                }

                sub = self.links[link].sub;
                if let Some(next_dep) = self.links[link].next_dep {
                    link = next_dep;
                    continue 'top;
                }
            }

            return dirty;
        }
    }

    /// Shallow propagation (mark dirty without full update)
    pub fn shallow_propagate(&mut self, link: LinkKey) {
        let mut link = link;
        loop {
            let Link { sub, next_sub, .. } = self.links[link];
            let flags = self.nodes[sub].flags;
            if (flags & (ReactiveFlags::PENDING | ReactiveFlags::DIRTY)) == ReactiveFlags::PENDING {
                self.nodes[sub].flags = flags | ReactiveFlags::DIRTY;
                if (flags & (ReactiveFlags::WATCHING | ReactiveFlags::RECURSED_CHECK))
                    == ReactiveFlags::WATCHING
                {
                    self.notify(sub);
                }
            }
            if let Some(next_sub) = next_sub {
                link = next_sub;
            } else {
                break;
            }
        }
    }

    /// Check if a link is still valid
    pub fn is_valid_link(&self, check_link: LinkKey, sub: NodeKey) -> bool {
        let mut link = self.nodes[sub].deps_tail;
        while let Some(link_key) = link {
            if link_key == check_link {
                return true;
            }
            link = self.links[link_key].prev_dep;
        }
        false
    }
}
