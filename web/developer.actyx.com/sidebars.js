module.exports = {
  homeSidebar: [
    {
      type: 'category',
      label: '💬 Explanations',
      collapsed: false,
      items: ['hello', 'event-streams', 'Test API Reference', 'node-lifecycle', 'page03', 'page03']
    },
    {
      type: 'category',
      label: '📘 How-To-Guides',
      collapsed: false,
      items: ['hello', 'event-streams', 'Test API Reference', 'node-lifecycle', 'page03', 'page03'],
    },
    {
      type: 'category',
      label: '🤓 API Reference',
      collapsed: false,
      items: ['hello',
        {
          type: 'category',
          label: 'Test Inner Sidebar',
          collapsed: false,
          items: ['hello', 'event-streams']
        }
      ],
    },
  ],
}
