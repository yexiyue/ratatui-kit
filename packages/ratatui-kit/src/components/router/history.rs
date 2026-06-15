use super::RouteContext;
use std::collections::VecDeque;

#[derive(Default, Clone)]
pub(crate) struct RouterHistory {
    pub current: usize,
    pub history: VecDeque<RouteContext>,
    pub max_length: usize,
}

impl RouterHistory {
    pub fn push(&mut self, context: RouteContext) {
        if self.history.len() >= self.max_length {
            self.history.pop_front();
            // 队首被移除后所有下标左移一位，`current` 必须同步回退，
            // 否则下面的 `current += 1` 会越过 `len`，`insert` 触发越界 panic
            // （历史栈达到 max_length 后继续 push 时复现）。
            self.current = self.current.saturating_sub(1);
        }
        self.current += 1;
        // 防御性夹取：插入位置最多到末尾（VecDeque::insert 允许 index == len 即追加，
        // 超过则 panic）。配合上面的回退，保证任何 push 都不越界。
        self.current = self.current.min(self.history.len());
        self.history.insert(self.current, context);
        self.history.truncate(self.current + 1);
    }

    pub fn replace(&mut self, route: RouteContext) {
        self.history[self.current] = route;
        self.history.truncate(self.current + 1);
    }

    pub fn back(&mut self) -> bool {
        if self.current > 0 {
            self.current -= 1;
            true
        } else {
            false
        }
    }

    pub fn forward(&mut self) -> bool {
        if self.current < self.history.len() - 1 {
            self.current += 1;
            true
        } else {
            false
        }
    }

    pub fn go(&mut self, n: i32) -> bool {
        let new_index = self.current as i32 + n;

        if new_index >= 0 && (new_index as usize) < self.history.len() {
            self.current = new_index as usize;
            true
        } else {
            false
        }
    }

    pub fn current_context(&self) -> RouteContext {
        self.history.get(self.current).unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(path: &str) -> RouteContext {
        RouteContext {
            path: path.to_string(),
            ..Default::default()
        }
    }

    fn new_history(max_length: usize) -> RouterHistory {
        let mut history = VecDeque::new();
        history.push_back(ctx("/"));
        RouterHistory {
            current: 0,
            history,
            max_length,
        }
    }

    /// 回归:历史栈达到 `max_length` 后继续 push 不再越界 panic
    /// （反复触发同一导航即复现旧 bug）。
    #[test]
    fn push_far_past_max_length_never_panics() {
        let mut h = new_history(10);
        for i in 0..100 {
            h.push(ctx(&format!("/r{i}")));
            assert!(h.current < h.history.len(), "current 越界: step {i}");
            assert!(h.history.len() <= h.max_length, "超出 max_length: step {i}");
        }
        // 始终指向最近一次 push 的路由。
        assert_eq!(h.current_context().path, "/r99");
    }

    /// 回归:在历史中部(back 之后)继续 push 同样不越界。
    #[test]
    fn back_then_push_stays_in_bounds() {
        let mut h = new_history(10);
        for i in 0..10 {
            h.push(ctx(&format!("/r{i}")));
        }
        for _ in 0..5 {
            h.back();
        }
        for i in 0..30 {
            h.push(ctx(&format!("/m{i}")));
            assert!(h.current < h.history.len(), "current 越界: step {i}");
        }
        assert_eq!(h.current_context().path, "/m29");
    }

    #[test]
    fn back_at_start_is_noop() {
        let mut h = new_history(10);
        // current == 0,无可回退。
        assert!(!h.back());
        assert_eq!(h.current, 0);
    }

    #[test]
    fn forward_at_end_is_noop() {
        let mut h = new_history(10);
        h.push(ctx("/a"));
        // 已在末尾。
        assert!(!h.forward());
        // 回退一格后可前进。
        assert!(h.back());
        assert!(h.forward());
        assert_eq!(h.current_context().path, "/a");
    }

    #[test]
    fn go_out_of_range_is_noop() {
        let mut h = new_history(10);
        for i in 0..3 {
            h.push(ctx(&format!("/r{i}")));
        }
        // 现有 4 项(含初始 "/"),current == 3。
        let before = h.current;
        assert!(!h.go(5)); // 越上界
        assert_eq!(h.current, before);
        assert!(!h.go(-10)); // 越下界
        assert_eq!(h.current, before);
        // 范围内成功。
        assert!(h.go(-2));
        assert_eq!(h.current, before - 2);
    }
}
