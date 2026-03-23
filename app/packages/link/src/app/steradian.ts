import localFont from 'next/font/local'

export const steradian = localFont({
  src: [
    {
      path: '../fonts/Steradian-Rg.otf',
      weight: '400',
      style: 'normal'
    },
    {
      path: '../fonts/Steradian-Md.otf',
      weight: '600',
      style: 'normal'
    },
    {
      path: '../fonts/Steradian-Bd.otf',
      weight: '700',
      style: 'normal'
    },
    {
      path: '../fonts/Steradian-Blk.otf',
      weight: '800',
      style: 'normal'
    }
  ]
})
