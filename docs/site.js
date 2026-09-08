(() => {
  const builtIn = location.pathname === '/docs' || location.pathname === '/docs.html';
  const base = builtIn ? 'https://neko233-com.github.io/proxysss/' : '';
  const header = document.createElement('header');
  header.className = 'docs-header';
  header.innerHTML = `<div class="docs-header-inner"><a class="docs-brand" href="${base}index.html">proxysss<small>使用文档</small></a><nav class="docs-nav" aria-label="文档主导航"><a href="${base}configuration.html">配置指南</a><a href="${base}architecture.html">工作原理</a><a href="https://github.com/neko233-com/proxysss">GitHub ↗</a><input class="docs-search" aria-label="筛选目录" placeholder="筛选本页目录…" type="search"></nav></div>`;
  document.body.prepend(header);
  const search = header.querySelector('input');
  search.addEventListener('input', () => {
    const query = search.value.trim().toLocaleLowerCase();
    document.querySelectorAll('.sidebar a').forEach(link => {
      link.classList.toggle('docs-hidden', query && !link.textContent.toLocaleLowerCase().includes(query));
    });
  });
  document.querySelectorAll('pre').forEach(pre => {
    const text = pre.textContent;
    const button = document.createElement('button');
    button.className = 'docs-copy'; button.type = 'button'; button.textContent = '复制';
    button.setAttribute('aria-label', '复制这段配置或命令');
    button.addEventListener('click', async () => {
      try { await navigator.clipboard.writeText(text); button.textContent = '已复制'; }
      catch { button.textContent = '请选中后复制'; }
      setTimeout(() => { button.textContent = '复制'; }, 1800);
    });
    pre.append(button);
  });
})();
