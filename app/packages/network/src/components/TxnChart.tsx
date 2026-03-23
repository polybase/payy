'use client'

import { Box } from '@chakra-ui/react'
import { getStats } from '../api'
import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Line } from 'react-chartjs-2'
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Filler,
  Tooltip,
  Legend
} from 'chart.js'

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Filler,
  Tooltip,
  Legend
)

export const options = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      display: false
    },
    title: {
      display: false
    }
  },
  elements: {
    point: {
      radius: 0
    },
    line: {
      tension: 0.4
    }
  },
  scales: {
    x: {
      grid: {
        lineWidth: 0,
        display: false
      },
      border: {
        display: false
      }
    },
    y: {
      border: {
        display: false
      },
      beginAtZero: true,
      position: 'right',
      ticks: {
        maxTicksLimit: 5,
        color: '#ffffff44'
      },
      grid: {
        color: '#ffffff05'
      }
    }
  }
}

export function TxnChart() {
  const query = useQuery({ queryKey: ['stats'], queryFn: getStats })
  const txns = query.data?.last_7_days_txns

  const gradient = useMemo(() => {
    if (typeof window === 'undefined') return
    const canvas = document?.createElement('canvas')
    const ctx = canvas?.getContext('2d')
    const gradient = ctx?.createLinearGradient(0, 0, 0, 200)
    gradient?.addColorStop(0, 'rgba(224, 255, 50, 0.2)') // the start color with 50% opacity
    gradient?.addColorStop(1, 'rgba(224, 255, 50, 0)')
    return gradient
  }, [])

  const dates = useMemo(() => {
    return txns?.map((txn) => new Date(txn.date)) ?? []
    // const now = new Date()
    // const start = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()))
    // return Array.from({ length: 7 }, (_, i) => new Date(+start - i * 24 * 60 * 60 * 1000)).reverse()
  }, [txns])

  const labels = useMemo(() => {
    return dates.map((date) =>
      date.toLocaleDateString('en-US', { day: '2-digit', month: '2-digit' })
    )
  }, [dates])

  const data = useMemo(() => {
    return {
      labels,
      datasets: [
        {
          label: 'Txns',
          fill: 'origin',
          data: txns?.map((txn) => txn.count) ?? [],
          borderColor: 'rgb(224, 255, 50)',
          backgroundColor: gradient ?? 'rgba(224, 255, 50, 0.2)'
        }
      ]
    }
  }, [labels, txns, gradient])

  return (
    <Box height="100%" width="100%" letterSpacing="5%" className="reset">
      <Line data={data} options={options as any} />
    </Box>
  )
}
