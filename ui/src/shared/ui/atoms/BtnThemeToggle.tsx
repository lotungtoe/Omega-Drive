import { Moon, Sun } from 'lucide-react'
import { cn } from '../../utils/index'

export function BtnThemeToggle({ dark, setDark }) {
  return (
    <button
      type="button"
      className={cn(
        'flex w-16 h-8 p-1 rounded-full cursor-pointer transition-all duration-300',
        dark ? 'bg-zinc-950 border border-zinc-800' : 'bg-white border border-zinc-200',
      )}
      onClick={() => setDark(!dark)}
    >
      <div className="flex justify-between items-center w-full">
        <div className={cn(
          'flex justify-center items-center w-6 h-6 rounded-full transition-transform duration-300',
          dark ? 'transform translate-x-0 bg-zinc-800' : 'transform translate-x-8 bg-gray-200',
        )}>
          {dark
            ? <Moon className="w-4 h-4 text-white" strokeWidth={1.5} />
            : <Sun className="w-4 h-4 text-gray-700" strokeWidth={1.5} />
          }
        </div>
        <div className={cn(
          'flex justify-center items-center w-6 h-6 rounded-full transition-transform duration-300',
          dark ? 'bg-transparent' : 'transform -translate-x-8',
        )}>
          {dark
            ? <Sun className="w-4 h-4 text-gray-500" strokeWidth={1.5} />
            : <Moon className="w-4 h-4 text-black" strokeWidth={1.5} />
          }
        </div>
      </div>
    </button>
  )
}

