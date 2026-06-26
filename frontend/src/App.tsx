import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getGateways, blockDevice, unblockDevice } from './api/client';
import { Wifi, WifiOff, Shield, ShieldOff, Gamepad2, Monitor, Smartphone } from 'lucide-react';

const GTA_ONLINE_IPS = [
  '23.45.67.89',
  '23.45.67.90',
];

function getDeviceIcon(hostname?: string) {
  if (!hostname) return <Monitor className="w-5 h-5" />;
  const h = hostname.toLowerCase();
  if (h.includes('ps5') || h.includes('playstation')) return <Gamepad2 className="w-5 h-5 text-blue-400" />;
  if (h.includes('xbox')) return <Gamepad2 className="w-5 h-5 text-green-400" />;
  if (h.includes('phone') || h.includes('iphone') || h.includes('android')) return <Smartphone className="w-5 h-5" />;
  return <Monitor className="w-5 h-5" />;
}

export default function App() {
  const queryClient = useQueryClient();

  const { data: gateways, isLoading, error } = useQuery({
    queryKey: ['gateways'],
    queryFn: getGateways,
    refetchInterval: 10000,
  });

  const blockMutation = useMutation({
    mutationFn: async ({ deviceMac, ruleId }: { deviceMac: string; ruleId: string }) => {
      return blockDevice(deviceMac, GTA_ONLINE_IPS, ruleId);
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['gateways'] }),
  });

  const unblockMutation = useMutation({
    mutationFn: async (ruleId: string) => unblockDevice(ruleId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['gateways'] }),
  });

  if (isLoading) {
    return (
      <div className="min-h-screen bg-gray-950 text-gray-100 flex items-center justify-center">
        <div className="animate-pulse text-lg">Loading gateways...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-gray-950 text-gray-100 flex items-center justify-center">
        <div className="text-red-400">Failed to load gateways. Is the backend running?</div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100">
      <header className="border-b border-gray-800 bg-gray-900/50 backdrop-blur sticky top-0 z-10">
        <div className="max-w-6xl mx-auto px-4 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Shield className="w-8 h-8 text-red-500" />
            <div>
              <h1 className="text-xl font-bold tracking-tight">anti-gav-ty</h1>
              <p className="text-xs text-gray-500">Network Control Panel</p>
            </div>
          </div>
          <div className="flex items-center gap-2 text-sm text-gray-400">
            <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            {gateways?.length || 0} gateways online
          </div>
        </div>
      </header>

      <main className="max-w-6xl mx-auto px-4 py-8">
        {!gateways || gateways.length === 0 ? (
          <div className="text-center py-20">
            <WifiOff className="w-16 h-16 mx-auto text-gray-600 mb-4" />
            <h2 className="text-xl text-gray-400 mb-2">No Gateways Found</h2>
            <p className="text-gray-600">Start the agent on your gateway machine to see it here.</p>
          </div>
        ) : (
          <div className="space-y-6">
            {gateways.map((gw) => (
              <div key={gw.id} className="bg-gray-900 border border-gray-800 rounded-xl overflow-hidden">
                <div className="p-4 flex items-center justify-between border-b border-gray-800">
                  <div className="flex items-center gap-3">
                    {gw.status === 'online' ? (
                      <Wifi className="w-5 h-5 text-green-500" />
                    ) : (
                      <WifiOff className="w-5 h-5 text-red-500" />
                    )}
                    <div>
                      <h3 className="font-semibold">{gw.name}</h3>
                      <p className="text-xs text-gray-500">
                        {gw.hostname} · {gw.mac_address}
                      </p>
                    </div>
                  </div>
                  <span
                    className={`px-2 py-1 rounded-full text-xs font-medium ${
                      gw.status === 'online'
                        ? 'bg-green-500/10 text-green-400'
                        : 'bg-red-500/10 text-red-400'
                    }`}
                  >
                    {gw.status}
                  </span>
                </div>

                <div className="p-4 border-b border-gray-800">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Gamepad2 className="w-4 h-4 text-orange-400" />
                      <span className="text-sm font-medium">GTA Online</span>
                    </div>
                    <div className="flex gap-2">
                      <button
                        onClick={() =>
                          blockMutation.mutate({
                            deviceMac: gw.mac_address,
                            ruleId: "gta5-" + gw.mac_address,
                          })
                        }
                        disabled={blockMutation.isPending}
                        className="flex items-center gap-1 px-3 py-1.5 bg-red-600 hover:bg-red-700 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors"
                      >
                        <ShieldOff className="w-4 h-4" />
                        Block
                      </button>
                      <button
                        onClick={() => unblockMutation.mutate("gta5-" + gw.mac_address)}
                        disabled={unblockMutation.isPending}
                        className="flex items-center gap-1 px-3 py-1.5 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors"
                      >
                        <Shield className="w-4 h-4" />
                        Unblock
                      </button>
                    </div>
                  </div>
                </div>

                <div className="p-4">
                  <h4 className="text-sm font-medium text-gray-400 mb-3">Devices on this gateway</h4>
                  <div className="text-sm text-gray-600 italic">
                    Device list coming in next update...
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}
