const remarkCodeImport = require('remark-code-import')

module.exports = {
  title: 'Actyx Developer',
  tagline: 'Documentation, guides and tools for building on the Actyx Platform',
  url: 'https://developer.actyx.com',
  baseUrl: '/',
  favicon: 'img/favicon.ico',
  organizationName: 'Actyx',
  projectName: 'Actyx Developer Website',
  onBrokenLinks: 'ignore',
  plugins: [
    [
      require.resolve('docusaurus-gtm-plugin'),
      {
        id: 'GTM-5PXCMFH',
      },
    ],
    [require.resolve('./src/plugins/analytics'), {}],
    [
      require.resolve('./src/plugins/arm'),
      {
        releasesYml: './__auto-releases.yml',
      },
    ],
  ],
  themeConfig: {
    announcementBar: {
      id: 'test007',
      content: 'Announcement Bar Content',
      backgroundColor: '#373c40',
      textColor: '#fff',
    },
    colorMode: {
      defaultMode: 'light',
      disableSwitch: true,
      respectPrefersColorScheme: false,
    },
    navbar: {
      title: '',
      logo: {
        alt: 'Actyx Developer',
        src: 'img/header.svg',
      },
      items: [
        {
          label: 'How-to Guides',
          activeBasePath: 'docs/how-to-guides',
          to: 'docs/how-to-guides/overview',
        },
        {
          label: 'Conceptual Guides',
          activeBasePath: 'docs/conceptual-guides',
          to: 'docs/conceptual-guides/overview',
        },
        {
          label: 'Reference',
          activeBasePath: 'docs/reference',
          to: 'docs/reference/overview',
        },
        {
          label: 'Tutorials',
          activeBasePath: 'docs/tutorials',
          to: 'docs/tutorials/overview',
        },
        {
          label: 'Blog',
          activeBasePath: '/blog',
          position: 'right',
          to: '/blog',
        },
        {
          label: 'Forum',
          position: 'right',
          to: 'https://community.actyx.com/',
        },
        /* {
          label: 'Community',
          position: 'right',
          items: [
            {
              label: 'Blog',
              href: '/blog',
            },
            {
              label: 'Forum',
              href: 'https://community.actyx.com/',
            },
            {
              label: 'Academy',
              href: 'https://community.actyx.com/',
            },
          ],
        }, */
      ],
    },
    footer: {
      logo: {
        alt: 'Actyx Developer',
        src: 'img/header.svg',
        href: 'https://developer.actyx.com',
      },
      style: 'dark',
      links: [
        {
          title: 'Reference Docs',
          items: [
            {
              label: 'Actyx',
              to: 'docs/reference/actyx-api',
            },
            {
              label: 'SDK',
              to: 'docs/reference/js-ts-sdk',
            },
            {
              label: 'CLI',
              to: 'docs/reference/cli',
            },
            {
              label: 'Node Manager',
              to: 'docs/reference/node-manager',
            },
          ],
        },
        {
          title: 'Start Building',
          items: [
            {
              label: 'Installing Actyx',
              to: 'docs/how-to-guides/local-developmentlocal-twins',
            },
            {
              label: 'Modelling in Twins',
              to: 'docs/how-to-guides/process-logic/modelling-processes-in-twins',
            },
            {
              label: 'Packaging UI Apps',
              to: 'docs/how-to-guides/packaging/mobile-apps',
            },
            {
              label: 'Actyx SDK',
              to: 'docs/how-to-guides/sdk/placeholder',
            },
          ],
        },
        {
          title: 'Essential Concepts',
          items: [
            {
              label: 'Event-based Systems',
              to: 'docs/conceptual-guides/event-based-systems',
            },
            {
              label: 'Local First',
              to: 'docs/conceptual-guides/local-first-cooperation',
            },
            {
              label: 'Thinking in Actyx',
              to: 'docs/conceptual-guides/thinking-in-actyx',
            },
            {
              label: 'Apps in Factories',
              to: 'docs/conceptual-guides/apps-in-the-factory-context',
            },
          ],
        },
        {
          title: 'For Developers',
          items: [
            {
              label: 'Forum',
              to: 'https://www.community.actyx.com',
            },
            {
              label: 'Discord',
              to: 'https://discord.gg/262yJhc',
            },
            {
              label: 'FAQ',
              to: 'docs/faq/supported-programming-languages',
            },
            {
              label: 'Blog',
              to: 'https://www.actyx.com/blog',
            },
          ],
        },
        {
          title: 'Company',
          items: [
            {
              label: 'Home',
              to: 'https://www.actyx.com',
            },
            {
              label: 'Team',
              to: 'https://www.actyx.com/team',
            },
            {
              label: 'Careers',
              to: 'https://www.actyx.com/careers',
            },
            {
              label: 'Press',
              to: 'https://www.actyx.com/blog',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Actyx AG`,
    },
    prism: {
      theme: require('prism-react-renderer/themes/vsDark'),
      darkTheme: require('prism-react-renderer/themes/dracula'),
      additionalLanguages: ['rust', 'csharp'],
    },
    algolia: {
      apiKey: 'dee14099c148f0ca14d046428003623a',
      indexName: 'actyx_developer',
      algoliaOptions: {}, // Optional, if provided by Algolia
    },
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          remarkPlugins: [remarkCodeImport],
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
        blog: {
          postsPerPage: 6,
          blogSidebarTitle: 'Our latest posts',
        },
      },
    ],
  ],
}
