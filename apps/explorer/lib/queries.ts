// React Query hooks for Seloria data fetching

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { api } from "./api";
import { ws } from "./websocket";

// Query keys
export const queryKeys = {
  status: ["status"] as const,
  block: (height: number) => ["block", height] as const,
  recentBlocks: (limit: number) => ["blocks", "recent", limit] as const,
  transaction: (hash: string) => ["transaction", hash] as const,
  account: (pubkey: string) => ["account", pubkey] as const,
  claim: (id: string) => ["claim", id] as const,
};

// Status query
export function useStatus() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.status,
    queryFn: () => api.getStatus(),
    refetchInterval: 5000, // Fallback polling
    staleTime: 2000,
  });

  // WebSocket updates
  useEffect(() => {
    const handleBlockFinalized = () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.status });
    };

    ws.on("BlockCommitted", handleBlockFinalized);
    return () => ws.off("BlockCommitted", handleBlockFinalized);
  }, [queryClient]);

  return query;
}

// Recent blocks query
export function useRecentBlocks(limit: number = 5) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.recentBlocks(limit),
    queryFn: () => api.getRecentBlocks(limit),
    staleTime: 2000,
  });

  // WebSocket updates
  useEffect(() => {
    const handleBlockFinalized = () => {
      queryClient.invalidateQueries({
        queryKey: ["blocks"],
      });
    };

    ws.on("BlockCommitted", handleBlockFinalized);
    return () => ws.off("BlockCommitted", handleBlockFinalized);
  }, [queryClient]);

  return query;
}

// Single block query
export function useBlock(height: number) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.block(height),
    queryFn: () => api.getBlock(height),
    enabled: height >= 0,
    staleTime: Infinity, // Blocks are immutable
  });

  return query;
}

// Transaction query
export function useTransaction(hash: string) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.transaction(hash),
    queryFn: () => api.getTransaction(hash),
    enabled: !!hash,
    staleTime: Infinity, // Transactions are immutable
  });

  return query;
}

// Account query
export function useAccount(pubkey: string) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.account(pubkey),
    queryFn: () => api.getAccount(pubkey),
    enabled: !!pubkey,
    staleTime: 2000,
  });

  // WebSocket updates
  useEffect(() => {
    const handleTxExecuted = () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.account(pubkey),
      });
    };

    ws.on("TxApplied", handleTxExecuted);
    return () => ws.off("TxApplied", handleTxExecuted);
  }, [queryClient, pubkey]);

  return query;
}

// Claim query
export function useClaim(id: string) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.claim(id),
    queryFn: () => api.getClaim(id),
    enabled: !!id,
    staleTime: 2000,
  });

  // WebSocket updates
  useEffect(() => {
    const handleClaimUpdated = (event: any) => {
      if (event.data.id === id || event.data.claim_id === id) {
        queryClient.invalidateQueries({
          queryKey: queryKeys.claim(id),
        });
      }
    };

    ws.on("ClaimFinalized", handleClaimUpdated);
    ws.on("AttestAdded", handleClaimUpdated);
    return () => {
      ws.off("ClaimFinalized", handleClaimUpdated);
      ws.off("AttestAdded", handleClaimUpdated);
    };
  }, [queryClient, id]);

  return query;
}
