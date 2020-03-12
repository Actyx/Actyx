/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { ArticleConfig } from './articleFish.support.test'

export const mockArticles: ArticleConfig[] = [
  {
    id: 'E9120356',
    description: 'Baysolvex D2EHPA',
    billOfMaterials: [],
    steps: [
      {
        workstationId: 'filling',
        instructions: [
          'Load tubes into workstation',
          'Setup machine according to setup parameters',
          'Turn on machine',
          'Load filled tubes into workpiece carrier',
        ],
        documentation: {
          location: '//cdn.mozilla.net/pdfjs/tracemonkey.pdf',
          page: 3,
        },
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'qa',
        instructions: [
          'Insert Tubes in Checking Machine',
          'Weight tube and check with target volume 3. Load into workpiece carrier if in target range',
        ],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'packaging',
        instructions: ['Take Tubes', 'Put them into box', 'Close Box'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
    ],
    documentationLink: '',
  },
  {
    id: 'E9120366',
    description: 'Aerosil R 202',
    billOfMaterials: [],
    steps: [
      {
        workstationId: 'machining',
        instructions: [
          'Load tubes into workstation',
          'Drill Holes as marked',
          'Load filled tubes into workpiece carrier',
        ],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'joining',
        instructions: ['Take Pipes', 'Align Pipes', 'Join Pipes'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'packaging',
        instructions: ['Take Tubes', 'Put them into box', 'Close Box'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
    ],
    documentationLink: '',
  },
  {
    id: 'E9120477',
    description: 'Glycerin 99,5',
    billOfMaterials: [],
    steps: [
      {
        workstationId: 'filling',
        instructions: [
          'Load tubes into workstation',
          'Setup machine according to setup parameters',
          'Turn on machine',
          'Load filled tubes into workpiece carrier',
        ],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'qa',
        instructions: [
          'Insert Tubes in Checking Machine',
          'Weight tube and check with target volume 3. Load into workpiece carrier if in target range',
        ],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'packaging',
        instructions: ['Take Tubes', 'Put them into box', 'Close Box'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
    ],
    documentationLink: '',
  },
  {
    id: 'E9221509',
    description: 'SLT-5130 SealWhite IDH666745',
    billOfMaterials: [],
    steps: [
      {
        workstationId: 'shaping',
        instructions: [
          'Insert materials in cold roll machine',
          'Start process',
          'Load material into workpiece carriers',
        ],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'machining',
        instructions: ['Take pipes', 'Drill holes as marked'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'coating',
        instructions: ['Put Pipes into galavanizing machine. ', 'Start machine'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
    ],
    documentationLink: '',
  },
  {
    id: 'E9321268',
    description: 'Eindrückdeckeleimer, 12l',
    billOfMaterials: [],
    steps: [
      {
        workstationId: 'filling',
        instructions: [
          'Load tubes into workstation',
          'Setup machine according to setup parameters',
          'Turn on machine',
          'Load filled tubes into workpiece carrier',
        ],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'qa',
        instructions: [
          'Insert Tubes in Checking Machine',
          'Weight tube and check with target volume',
          'Load into workpiece carrier if in target range',
        ],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'packaging',
        instructions: ['Take Tubes', 'Put them into box', 'Close Box'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
    ],
    documentationLink: '',
  },
  {
    id: 'K9200137',
    description: 'Delo-Duopox 1895 Komp. A',
    billOfMaterials: [],
    steps: [
      {
        workstationId: 'filling',
        instructions: ['Take tubes', 'Fill tubes'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'qa',
        instructions: ['Insert Tubes in Checking Machine', 'Check filling'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
      {
        workstationId: 'packaging',
        instructions: ['Take Tubes', 'Put them into box', 'Close Box'],
        totals: {
          productionTime: 0,
          pauseTime: 0,
          produced: 0,
          scrap: 0,
        },
      },
    ],
    documentationLink: '',
  },
]
