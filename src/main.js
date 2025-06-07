document.addEventListener('DOMContentLoaded', () => {
  const list = document.getElementById('mod-list');
  new Sortable(list, {
    animation: 150,
    ghostClass: 'sortable-ghost',
    forceFallback: true,
    onEnd: (evt) => {
      console.log(`Moved item from ${evt.oldIndex} to ${evt.newIndex}`);
      const items = Array.from(list.children).map(item => item.textContent);
      console.log('New order:', items);
    },
    onMove: () => true,
  });

  let currentView = 'main';
  window.changeView = function(view) {
    document.getElementById('main-page').style.display = view === 'main' ? 'block' : 'none';
    document.getElementById('settings-page').style.display = view === 'settings' ? 'block' : 'none';
    currentView = view;
  };

  const contextMenu = document.getElementById('context-menu');
  list.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    contextMenu.style.display = 'block';
    contextMenu.style.left = e.pageX + 'px';
    contextMenu.style.top = e.pageY + 'px';
  });

  document.addEventListener('click', () => contextMenu.style.display = 'none');
});