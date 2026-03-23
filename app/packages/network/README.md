# Payy Network - ZK-Rollup Blockchain Explorer

A Next.js application that serves as the blockchain explorer and network monitoring dashboard for Payy Network, the first mainnet ZK-ZK rollup.

🌐 **Live Site**: [https://payy.network](https://payy.network)

## Overview

Payy Network is a zero-knowledge rollup blockchain explorer that provides real-time network statistics, transaction monitoring, and diagnostic tools for the ZK-rollup infrastructure. The application serves as the public-facing interface for monitoring network health, transaction activity, and blockchain data.

## Key Features

### 🏠 **Landing Page** (`/`)
- Network branding: "THE FIRST MAINNET ZK-ZK ROLLUP"
- Real-time network statistics display
- Average block time monitoring
- Sequencer and rollup height tracking

### 🔍 **Blockchain Explorer** (`/explorer`)
- Interactive network statistics dashboard
- Transaction activity charts and visualizations
- Block and transaction monitoring panels
- Real-time data updates with Chart.js integration

### 🔧 **Diagnostics** (`/diagnostics`)
- Detailed transaction analysis tools
- Wallet activity monitoring
- Note reconstruction and difference analysis
- Advanced network diagnostic capabilities
- CodeMirror-based code editors for transaction inspection

## Technology Stack

- **Framework**: Next.js 15.x with React 19 and TypeScript
- **UI Library**: Chakra UI v2 (React 19 compatible) with Framer Motion animations
- **Data Visualization**: Chart.js with React bindings
- **Data Fetching**: TanStack Query for API state management
- **HTTP Client**: Axios for API communications
- **Code Editor**: CodeMirror for transaction inspection

## Internal Dependencies

The Network application depends on two core services from the ZK-rollup infrastructure:

### 📦 **pkg/guild**
The Guild service provides API endpoints for additional services above the protocol layer, including:
- Contract status and blockchain data
- User rewards and points system
- Wallet management and authentication
- Registry and note operations

### 🔗 **pkg/node**
The Node service is the primary blockchain client that provides:
- Real-time network health metrics
- Transaction and block data
- Rollup height and sequencer information
- Network performance statistics

## API Integration

The application communicates with these internal services through their API endpoints:

- **Guild API** (`NEXT_PUBLIC_GUILD_URL`): Connects to the `pkg/guild` service for contract status, blockchain data, and additional services
- **Rollup API** (`NEXT_PUBLIC_ROLLUP_URL`): Connects to the `pkg/node` service for network health and performance metrics

## Getting Started

### Prerequisites

- Node.js 18+
- Yarn (workspace manager)

### Installation

```bash
# From the app/ directory
yarn install
```

### Development

```bash
# From the app/ directory
yarn workspace network dev
```

Open [http://localhost:3000](http://localhost:3000) to view the application.

### Building

```bash
# From the app/ directory
yarn workspace network build
```

## Code Quality

This package follows the workspace ESLint configuration and TypeScript standards. Always run these commands after making changes:

```bash
cd app
yarn lint
yarn lint:types
yarn test
```

## Environment Variables

Configure the following environment variables to connect to the internal services:

```bash
# Guild API endpoint - connects to pkg/guild service
# Provides contract status, blockchain data, and user services
NEXT_PUBLIC_GUILD_URL=your_guild_api_url

# Rollup API endpoint - connects to pkg/node service  
# Provides network health metrics and blockchain performance data
NEXT_PUBLIC_ROLLUP_URL=your_rollup_api_url
```

### Local Development

For local development, you typically run these services separately:

1. Start the Guild service: See [pkg/guild/README.md](../../../pkg/guild/README.md)
2. Start the Node service: See [pkg/node/README.md](../../../pkg/node/README.md)
3. Configure the environment variables to point to your local instances

## Network Statistics

The explorer displays key network metrics:

- **Average Block Time**: Real-time block production rate
- **Sequencer Height**: Current sequencer block height
- **Rollup Height**: Current rollup block height
- **Transaction Volume**: Charts showing transaction activity over time

## Deployment

The application is deployed on Vercel and accessible at [https://payy.network](https://payy.network).

For deployment configuration, see the Next.js deployment documentation.
