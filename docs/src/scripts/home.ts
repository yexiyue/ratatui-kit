function initHome() {
  const root = document.querySelector<HTMLElement>(".rk-home");
  if (!root || root.dataset.ready) return;
  root.dataset.ready = "true";
  document
    .querySelector("astro-theme-toggle")
    ?.setAttribute(
      "aria-label",
      document.documentElement.lang.startsWith("zh")
        ? "切换明暗主题"
        : "Switch color theme",
    );
  const tabs = Array.from(
    root.querySelectorAll<HTMLButtonElement>('[role="tab"]'),
  );
  function select(tab: HTMLButtonElement) {
    for (const item of tabs) {
      const active = item === tab;
      item.setAttribute("aria-selected", String(active));
      item.tabIndex = active ? 0 : -1;
      const panel = document.getElementById(
        item.getAttribute("aria-controls")!,
      );
      if (panel) {
        panel.hidden = !active;
        const img = panel.querySelector<HTMLImageElement>("img");
        if (!active && img) {
          img.src = img.dataset.poster!;
          const play = panel.querySelector<HTMLButtonElement>(".rk-play");
          play?.setAttribute("aria-pressed", "false");
          if (play) play.textContent = play.dataset.play!;
        }
      }
    }
  }
  root.querySelectorAll<HTMLButtonElement>(".rk-play").forEach((button) => {
    button.addEventListener("click", () => {
      const img = button
        .closest("[role=tabpanel]")!
        .querySelector<HTMLImageElement>("img")!;
      const playing = button.getAttribute("aria-pressed") !== "true";
      img.src = playing ? img.dataset.src! : img.dataset.poster!;
      button.setAttribute("aria-pressed", String(playing));
      button.textContent = playing
        ? button.dataset.pause!
        : button.dataset.play!;
    });
  });
  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => select(tab));
    tab.addEventListener("keydown", (event) => {
      let next = index;
      if (event.key === "ArrowRight") next = (index + 1) % tabs.length;
      else if (event.key === "ArrowLeft")
        next = (index + tabs.length - 1) % tabs.length;
      else if (event.key === "Home") next = 0;
      else if (event.key === "End") next = tabs.length - 1;
      else return;
      event.preventDefault();
      select(tabs[next]);
      tabs[next].focus();
    });
  });
  root
    .querySelector<HTMLButtonElement>("[data-copy]")
    ?.addEventListener("click", async (event) => {
      const button = event.currentTarget as HTMLButtonElement;
      const status = root.querySelector<HTMLElement>(".rk-copy-status");
      try {
        await navigator.clipboard.writeText(button.dataset.copy!);
        if (status) status.textContent = button.dataset.copied!;
      } catch {
        if (status) status.textContent = button.dataset.failed!;
      }
    });
}
initHome();
document.addEventListener("astro:page-load", initHome);
