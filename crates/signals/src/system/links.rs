use crate::types::{Link, LinkKey, NodeKey};

impl super::ReactiveSystem {
    /// Remove all dependency links from a subscriber
    pub fn purge_deps(&mut self, sub: NodeKey, purge_tail: bool) {
        let (deps, deps_tail) = match self.nodes.get(sub) {
            Some(n) => (n.deps, n.deps_tail),
            None => return,
        };

        // If purge_tail is true, start from deps (purge tail)
        // Otherwise, start from deps_tail.next_dep (skips tail, original behavior)
        let mut current = if purge_tail {
            deps
        } else {
            deps_tail
                .and_then(|tail| self.links.get(tail))
                .map(|l| l.next_dep)
                .unwrap_or(deps)
        };

        while let Some(dep_key) = current {
            current = self.links[dep_key].next_dep;
            self.unlink(dep_key);
        }
    }

    /// Remove all subscriber links from a dependency
    #[inline]
    pub fn purge_subs(&mut self, dep: NodeKey) {
        // Iterate from tail to head using prev_sub to avoid issues with deps_tail updates
        let mut current = self.nodes[dep].subs;
        while let Some(sub_key) = current {
            current = self.links[sub_key].next_sub;
            self.unlink(sub_key);
        }
    }

    /// Create a link between a dependency and a subscriber
    pub fn link(&mut self, dep: NodeKey, sub: NodeKey, version: usize) {
        let prev_dep = self.nodes[sub].deps_tail;
        if let Some(prev_dep) = prev_dep
            && self.links[prev_dep].dep == dep
        {
            return;
        }
        let next_dep = if let Some(prev_dep) = prev_dep {
            self.links[prev_dep].next_dep
        } else {
            self.nodes[sub].deps
        };
        if let Some(next_dep) = next_dep
            && self.links[next_dep].dep == dep
        {
            self.links[next_dep].version = version;
            self.nodes[sub].deps_tail = Some(next_dep);
            return;
        }
        let prev_sub = self.nodes[dep].subs_tail;
        if let Some(prev_sub) = prev_sub
            && self.links[prev_sub].version == version
            && self.links[prev_sub].sub == sub
        {
            return;
        }

        let new_link = self.links.insert(Link {
            version,
            dep,
            sub,
            prev_dep,
            next_dep,
            prev_sub,
            next_sub: None,
        });

        self.nodes[sub].deps_tail = Some(new_link);
        self.nodes[dep].subs_tail = Some(new_link);

        if let Some(next_dep) = next_dep {
            self.links[next_dep].prev_dep = Some(new_link);
        }
        if let Some(prev_dep) = prev_dep {
            self.links[prev_dep].next_dep = Some(new_link);
        } else {
            self.nodes[sub].deps = Some(new_link);
        }
        if let Some(prev_sub) = prev_sub {
            self.links[prev_sub].next_sub = Some(new_link);
        } else {
            self.nodes[dep].subs = Some(new_link);
        }
    }

    /// Core unlink logic: removes a link and updates all adjacent pointers
    /// Returns (next_dep, next_sub) for iteration purposes
    pub fn unlink(&mut self, link: LinkKey) {
        let Self { nodes, links, .. } = self;
        let Some(Link {
            dep,
            sub,
            prev_sub,
            next_sub,
            prev_dep,
            next_dep,
            ..
        }) = links.remove(link)
        else {
            return;
        };

        // Update dep list in subscriber node
        if let Some(next_dep) = next_dep {
            links[next_dep].prev_dep = prev_dep;
        } else {
            nodes[sub].deps_tail = prev_dep;
        }

        if let Some(prev_dep) = prev_dep {
            links[prev_dep].next_dep = next_dep;
        } else {
            nodes[sub].deps = next_dep;
        }

        // Update sub list in dependency node
        if let Some(next_sub) = next_sub {
            links[next_sub].prev_sub = prev_sub;
        } else {
            nodes[dep].subs_tail = prev_sub;
        }

        if let Some(prev_sub) = prev_sub {
            links[prev_sub].next_sub = next_sub;
        } else {
            nodes[dep].subs = next_sub;
            if next_sub.is_none() {
                self.unwatched(dep);
            }
        }
    }
}
