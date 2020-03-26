module.exports = {
  title: 'Actyx Developer',
  tagline: 'Documentation, guides and tools for building on the Actyx Platform',
  url: 'https://developer.actyx.com',
  baseUrl: '/',
  favicon: 'img/favicon.ico',
  organizationName: 'Actyx',
  projectName: 'Actyx',
  themeConfig: {
    navbar: {
      title: 'Actyx Developers',
      logo: {
        alt: 'Actyx Developer',
        src: 'img/logo.svg',
      },
      links: [
        {
          to: 'docs/os/introduction',
          activeBasePath: 'docs/os/',
          label: 'ActyxOS',
          position: 'left',
        },
        {
          to: 'docs/pond/introduction',
          activeBasePath: 'docs/pond/',
          label: 'Actyx Pond',
          position: 'left',
        },
        {
          to: 'docs/cli',
          activeBasePath: 'docs/cli',
          label: 'Actyx CLI',
          position: 'left',
        },
        {
          to: 'docs/tutorials/doc1',
          activeBasePath: 'docs/tutorials',
          label: 'Tutorials',
          position: 'left',
        },
        {
          to: 'docs/faq/supported-programming-languages',
          activeBasePath: 'docs/faq/',
          label: 'FAQs',
          position: 'left',
        },
        {
          to: 'https://downloads.actyx.com',
          label: 'Downloads',
          position: 'right',
        },
        {
          to: 'https://www.actyx.com',
          label: 'www.actyx.com',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'light',
      copyright: `Copyright © ${new Date().getFullYear()} Actyx AG`,
    },
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],
};
