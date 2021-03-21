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
          label: 'Reference Docs',
          activeBasePath: 'docs/reference',
          to: 'docs/reference/overview',
        },
        {
          label: 'Blog',
          to: 'blog',
          position: 'right',
        },
        {
          label: 'Community',
          to: 'https://community.actyx.com/',
          position: 'right',
        },
        {
          label: 'Academy',
          to: 'https://community.actyx.com/',
          position: 'right',
        },
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
              to: 'docs/build/reference/actyx-api',
            },
            {
              label: 'SDK',
              to: 'docs/build/reference/js-ts-sdk',
            },
            {
              label: 'CLI',
              to: 'docs/deploy/reference/cli',
            },
            {
              label: 'Node Manager',
              to: 'docs/deploy/reference/cli',
            },
          ],
        },
        {
          title: 'Building Solutions',
          items: [
            {
              label: 'Local Twins',
              to: 'docs/build/explanation/local-twins',
            },
            {
              label: 'UI Apps',
              to: 'docs/build/explanation/ui-apps',
            },
            {
              label: 'Headless Apps',
              to: 'docs/build/explanation/headless-apps',
            },
            {
              label: 'Actyx SDK',
              to: 'docs/build/how-to/actyx-sdk',
            },
          ],
        },
        {
          title: 'Quick Links',
          items: [
            {
              label: 'Actyx Node',
              to: 'docs/start/explanation/the-actyx-node',
            },
            {
              label: 'Typescript SDK',
              to: 'docs/build/reference/js-ts-sdk',
            },
            {
              label: 'Node Configuration',
              to: 'docs/deploy/how-to/node-configuration',
            },
            {
              label: 'Deployments',
              to: 'docs/deploy/how-to/deployments',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'Forum',
              to: 'https://www.actyx.com',
            },
            {
              label: 'Actyx Academy',
              to: 'https://www.actyx.com/team',
            },
            {
              label: 'Discord',
              to: 'https://www.actyx.com/careers',
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
