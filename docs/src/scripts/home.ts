/*
	rk-home behaviors, shared by both locales.

	1. the hero terminal: examples/apps/todo_app.rs rebuilt as a live DOM app —
	   same keybinds (j/k move, space toggle, f filter, d delete-with-confirm,
	   a add, q quit), same initial tasks, same status strings. It idles through
	   a slow autoplay until the visitor takes over, then it is theirs.
	2. the component browser: one pane shape, j/k or click to switch.
	3. the reactive ripple: one `count += 1` write, every dependent updates.
	4. the install-line copy button.
*/

type Todo = { title: string; done: boolean };
type Filter = 'all' | 'open' | 'done';

const FILTER_ORDER: Filter[] = ['all', 'open', 'done'];

function setupTerm(root: HTMLElement) {
	const app = root.querySelector<HTMLElement>('[data-rk-term-app]');
	const listEl = root.querySelector<HTMLElement>('[data-rk-list]');
	const emptyEl = root.querySelector<HTMLElement>('[data-rk-empty]');
	const filterEl = root.querySelector<HTMLElement>('[data-rk-filter]');
	const openEl = root.querySelector<HTMLElement>('[data-rk-open]');
	const doneEl = root.querySelector<HTMLElement>('[data-rk-done]');
	const selectedEl = root.querySelector<HTMLElement>('[data-rk-selected]');
	const eventEl = root.querySelector<HTMLElement>('[data-rk-event]');
	const barStatusEl = root.querySelector<HTMLElement>('[data-rk-term-status]');
	const searchPrompt = root.querySelector<HTMLElement>('[data-rk-search-prompt]');
	const searchSlot = root.querySelector<HTMLElement>('[data-rk-search-slot]');
	const modal = root.querySelector<HTMLElement>('[data-rk-modal]');
	const modalText = root.querySelector<HTMLElement>('[data-rk-modal-text]');
	const modalDelete = root.querySelector<HTMLButtonElement>('[data-rk-modal-del]');
	const modalKeep = root.querySelector<HTMLButtonElement>('[data-rk-modal-keep]');
	const exitOverlay = root.querySelector<HTMLElement>('[data-rk-exit]');
	if (!app || !listEl) return;

	const initialTodos = (): Todo[] => {
		const source = listEl.getAttribute('data-initial') ?? '';
		return source
			.split('|')
			.filter(Boolean)
			.map((pair) => {
				const [title, flag] = pair.split('~');
				return { title, done: flag === '1' };
			});
	};

	let todos: Todo[] = initialTodos();
	let cursor = 0;
	let filter: Filter = 'all';
	let status = 'ready';
	let exited = false;
	let modalChoice: 'delete' | 'keep' = 'delete';
	let searchActive = false;

	const visible = () =>
		todos
			.map((todo, index) => ({ todo, index }))
			.filter(({ todo }) =>
				filter === 'all' ? true : filter === 'open' ? !todo.done : todo.done,
			);

	const clampCursor = () => {
		const indices = visible().map((v) => v.index);
		if (!indices.includes(cursor)) cursor = indices[0] ?? 0;
	};

	const setStatus = (next: string) => {
		status = next;
	};

	const makeRow = ({ todo, index }: { todo: Todo; index: number }) => {
		const li = document.createElement('li');
		li.className = 'rk-task' + (todo.done ? ' rk-task--done' : '');
		li.setAttribute('role', 'option');
		li.setAttribute('aria-selected', index === cursor ? 'true' : 'false');

		const marker = document.createElement('span');
		marker.className = 'rk-task__marker';
		marker.textContent = '>';

		const box = document.createElement('span');
		box.className = 'rk-task__box';
		box.textContent = todo.done ? '[x]' : '[ ]';

		const title = document.createElement('span');
		title.className = 'rk-task__title';
		title.textContent = todo.title;

		li.append(marker, box, title);
		li.addEventListener('click', () => {
			cursor = index;
			render();
			app?.focus();
		});
		return li;
	};

	const render = () => {
		listEl.replaceChildren(...visible().map(makeRow));
		if (emptyEl) emptyEl.hidden = visible().length > 0;
		if (filterEl) filterEl.textContent = filter;
		const openCount = todos.filter((todo) => !todo.done).length;
		if (openEl) openEl.textContent = String(openCount);
		if (doneEl) doneEl.textContent = String(todos.length - openCount);
		if (selectedEl)
			selectedEl.textContent = todos[cursor]?.title ?? '<none>';
		if (eventEl) eventEl.textContent = status;
		if (barStatusEl)
			barStatusEl.textContent = `tasks ${todos.length} · ${filter}`;
	};

	/* ---- search input (the `a` key, like SearchInput in the source) ---- */
	const openSearch = () => {
		if (searchActive || exited) return;
		searchActive = true;
		const input = document.createElement('input');
		input.className = 'rk-search__input';
		input.maxLength = 48;
		input.addEventListener('keydown', (event) => {
			event.stopPropagation();
			if (event.key === 'Enter') {
				const title = input.value.trim();
				if (title.length < 3) {
					setStatus('task title needs at least 3 chars');
					render();
					return;
				}
				todos.push({ title, done: false });
				filter = 'all';
				cursor = todos.length - 1;
				setStatus(`added: ${title}`);
				closeSearch();
				render();
			} else if (event.key === 'Escape') {
				closeSearch();
				setStatus('ready');
				render();
				app?.focus();
			}
		});
		if (searchPrompt) searchPrompt.hidden = true;
		searchSlot?.append(input);
		input.focus();
	};

	const closeSearch = () => {
		searchActive = false;
		searchSlot?.querySelector('input')?.remove();
		if (searchPrompt) searchPrompt.hidden = false;
	};

	/* ---- confirm modal (the `d` key, ConfirmModal in the source) ---- */
	const openModal = () => {
		if (todos.length === 0) {
			setStatus('nothing to delete');
			render();
			return;
		}
		modalChoice = 'delete';
		if (modalText)
			modalText.textContent = `Remove ${JSON.stringify(todos[cursor]?.title ?? '')}?`;
		if (modal) modal.hidden = false;
		modalDelete?.setAttribute('aria-selected', 'true');
		modalKeep?.setAttribute('aria-selected', 'false');
		setStatus('delete confirmation opened');
		render();
	};

	const closeModal = (confirm: boolean) => {
		if (confirm && modal) {
			const removed = todos.splice(cursor, 1)[0];
			if (todos.length === 0) cursor = 0;
			else clampCursor();
			setStatus(removed ? `deleted: ${removed.title}` : 'task already gone');
		} else {
			setStatus('delete canceled');
		}
		if (modal) modal.hidden = true;
		render();
		app?.focus();
	};

	modalDelete?.addEventListener('click', () => closeModal(true));
	modalKeep?.addEventListener('click', () => closeModal(false));

	/* ---- exit overlay (the `q` key) ---- */
	const restart = () => {
		todos = initialTodos();
		cursor = 0;
		filter = 'all';
		exited = false;
		if (exitOverlay) exitOverlay.hidden = true;
		setStatus('ready');
		render();
		app?.focus();
	};

	/* ---- key routing ---- */
	app.addEventListener('keydown', (event) => {
		if (exited) {
			if (event.key === 'Enter') restart();
			return;
		}
		if (searchActive) return; // the input layer owns keys while active

		const key = event.key.toLowerCase();
		switch (key) {
			case 'q':
				exited = true;
				if (exitOverlay) exitOverlay.hidden = false;
				break;
			case 'j':
			case 'arrowdown': {
				const indices = visible().map((v) => v.index);
				const at = indices.indexOf(cursor);
				cursor = indices[Math.min(at + 1, indices.length - 1)] ?? 0;
				break;
			}
			case 'k':
			case 'arrowup': {
				const indices = visible().map((v) => v.index);
				const at = indices.indexOf(cursor);
				cursor = indices[Math.max(at - 1, 0)] ?? 0;
				break;
			}
			case ' ':
			case 'enter': {
				const todo = todos[cursor];
				if (!todo) break;
				todo.done = !todo.done;
				setStatus(`${todo.done ? 'done' : 'open'}: ${todo.title}`);
				break;
			}
			case 'f': {
				filter = FILTER_ORDER[(FILTER_ORDER.indexOf(filter) + 1) % 3];
				clampCursor();
				setStatus(`filter: ${filter}`);
				break;
			}
			case 'd':
				openModal();
				break;
			case 'a':
				openSearch();
				break;
			default:
				return;
		}
		event.preventDefault();
		render();
	});

	/* modal buttons answer to arrows/enter while open */
	app.addEventListener('keydown', (event) => {
		if (modal?.hidden !== false || exited) return;
		if (['arrowleft', 'arrowright', 'tab', 'enter', 'escape'].includes(event.key.toLowerCase())) {
			event.preventDefault();
			event.stopPropagation();
			if (event.key.toLowerCase() === 'escape') {
				closeModal(false);
			} else if (event.key === 'Enter') {
				closeModal(modalChoice === 'delete');
			} else {
				modalChoice = modalChoice === 'delete' ? 'keep' : 'delete';
				modalDelete?.setAttribute(
					'aria-selected',
					modalChoice === 'delete' ? 'true' : 'false',
				);
				modalKeep?.setAttribute(
					'aria-selected',
					modalChoice === 'keep' ? 'true' : 'false',
				);
			}
		}
	});

	render();

	/* ---- idle autoplay: the app demos itself until taken over ---- */
	const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	let interacted = reduce;
	let tick = 0;

	const step = () => {
		const indices = visible().map((v) => v.index);
		if (indices.length === 0) return;
		const at = indices.indexOf(cursor);
		if (tick % 3 === 2) {
			const todo = todos[cursor];
			if (todo) {
				todo.done = !todo.done;
				setStatus(`${todo.done ? 'done' : 'open'}: ${todo.title}`);
			}
		} else {
			cursor = indices[(at + 1) % indices.length];
		}
		tick += 1;
		render();
	};

	if (!interacted) {
		let inView = true;
		const io = new IntersectionObserver(
			(entries) => {
				inView = entries[0]?.isIntersecting ?? false;
			},
			{ threshold: 0.25 },
		);
		io.observe(app);

		const timer = window.setInterval(() => {
			if (!interacted && inView && !document.hidden && modal?.hidden !== false && !searchActive && !exited) {
				step();
			}
		}, 2600);

		const stop = () => {
			interacted = true;
			window.clearInterval(timer);
			io.disconnect();
		};
		root.addEventListener('pointerdown', stop, { once: true });
		app.addEventListener('focusin', stop, { once: true });
	}
}

function setupBrowser(root: HTMLElement) {
	const browser = root.querySelector<HTMLElement>('[data-rk-browser]');
	if (!browser) return;

	const items = Array.from(
		browser.querySelectorAll<HTMLButtonElement>('.rk-browser__item'),
	);
	const panels = Array.from(
		browser.querySelectorAll<HTMLElement>('.rk-browser__panel'),
	);

	const select = (index: number) => {
		items.forEach((item, i) =>
			item.setAttribute('aria-selected', i === index ? 'true' : 'false'),
		);
		panels.forEach((panel, i) =>
			panel.classList.toggle('is-active', i === index),
		);
	};

	items.forEach((item, index) => {
		item.addEventListener('click', () => select(index));
	});

	browser.addEventListener('keydown', (event) => {
		const key = event.key.toLowerCase();
		const current = items.findIndex(
			(item) => item.getAttribute('aria-selected') === 'true',
		);
		if (key !== 'j' && key !== 'arrowdown' && key !== 'k' && key !== 'arrowup') return;
		event.preventDefault();
		const delta = key === 'j' || key === 'arrowdown' ? 1 : -1;
		const next = (current + delta + items.length) % items.length;
		select(next);
		items[next]?.focus();
	});
}

function setupRipple(root: HTMLElement) {
	const button = root.querySelector<HTMLButtonElement>('[data-rk-ripple-btn]');
	const nodes = root.querySelectorAll<HTMLElement>('[data-rk-count]');
	if (!button || nodes.length === 0) return;

	let count = 7;
	button.addEventListener('click', () => {
		count += 1;
		nodes.forEach((node) => {
			node.textContent = String(count).padStart(2, '0');
		});
	});
}

function setupCopy(root: HTMLElement) {
	const button = root.querySelector<HTMLButtonElement>('[data-rk-copy]');
	if (!button) return;
	const cmd = button.getAttribute('data-cmd') ?? 'cargo add ratatui-kit';
	const rest = button.getAttribute('data-rest') ?? 'copied';
	const idle = button.textContent ?? 'copy';

	button.addEventListener('click', async () => {
		try {
			await navigator.clipboard.writeText(cmd);
			button.textContent = rest;
			button.classList.add('is-copied');
			window.setTimeout(() => {
				button.textContent = idle;
				button.classList.remove('is-copied');
			}, 1600);
		} catch {
			button.textContent = cmd;
			window.setTimeout(() => {
				button.textContent = idle;
			}, 1600);
		}
	});
}

const boot = () => {
	document.querySelectorAll<HTMLElement>('[data-rk-home]').forEach((root) => {
		setupTerm(root);
		setupBrowser(root);
		setupRipple(root);
		setupCopy(root);
	});
};

if (document.readyState === 'loading') {
	document.addEventListener('DOMContentLoaded', boot);
} else {
	boot();
}
