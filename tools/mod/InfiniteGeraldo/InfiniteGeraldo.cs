// InfiniteGeraldo: a MelonLoader mod that gives Geraldo's shop infinite stock. Items still cost
// cash, but buying one never uses up stock, so each item can be bought any number of times per
// round. Tower buffs (Hot Sauce, Pickles, Sharpening Stone, Fertilizer, See Invisibility...) stack:
// any number of them can go on the same tower. Build and install: MOD=InfiniteGeraldo sh tools/mod/build.sh
using System;
using HarmonyLib;
using Il2CppAssets.Scripts;
using Il2CppAssets.Scripts.Models;
using Il2CppAssets.Scripts.Simulation.Input;
using Il2CppAssets.Scripts.Simulation.Objects;
using Il2CppAssets.Scripts.Simulation.Powers;
using MelonLoader;

[assembly: MelonInfo(typeof(InfiniteGeraldo.Mod), "InfiniteGeraldo", "1.0.0", "rrbtd6")]
[assembly: MelonGame("Ninja Kiwi", "BloonsTD6")]
[assembly: MelonGame("Ninja Kiwi", "BloonsTD6-Epic")]

namespace InfiniteGeraldo;

public class Mod : MelonMod
{
    public override void OnInitializeMelon() => LoggerInstance.Msg("Geraldo's shop has infinite stock; item buffs stack.");

    /// Keeps an item showing at least one in stock, for UI that reads `remaining` directly.
    internal static void KeepStocked(GeraldoShopInventory.GeraldoStockItem item)
    {
        if (item != null && item.remaining < 1)
            item.remaining = 1;
    }
}

/// A purchase deducts nothing (and still reports success, so the purchase goes through).
[HarmonyPatch(typeof(GeraldoShopInventory), nameof(GeraldoShopInventory.DeductStock))]
static class DeductStockPatch
{
    static bool Prefix(GeraldoShopInventory __instance, string name, ref bool __result)
    {
        Mod.KeepStocked(__instance.GetStockItem(name));
        __result = true;
        return false;
    }
}

/// Every shop item is always in stock (HasStock is GetStockItem(name).remaining > 0; the game's
/// CanPurchase is cash >= cost && HasStock, so the price still applies).
[HarmonyPatch(typeof(GeraldoShopInventory), nameof(GeraldoShopInventory.HasStock))]
static class HasStockPatch
{
    static void Postfix(GeraldoShopInventory __instance, string name, ref bool __result) =>
        __result = __instance.GetStockItem(name) != null;
}

/// No per-game purchase caps.
[HarmonyPatch(typeof(GeraldoShopInventory), nameof(GeraldoShopInventory.HasHitMaxPurchaseLimit))]
static class MaxPurchaseLimitPatch
{
    static void Postfix(ref bool __result) => __result = false;
}

/// The shop's stock count never shows empty.
[HarmonyPatch(typeof(GeraldoShopInventory), nameof(GeraldoShopInventory.GetStockItem))]
static class GetStockItemPatch
{
    static void Postfix(GeraldoShopInventory.GeraldoStockItem __result) => Mod.KeepStocked(__result);
}

/// While a Geraldo item is being bought (GeraldoPurchaseManager.Activate), every buff it adds is a
/// fresh copy under its own id. The items only look for their buff by id: IsMutatedBy(id) blocks
/// a second Hot Sauce / Fertilizer / Sharpening Stone, GetMutatorByKey (id.Contains(key)) blocks a
/// second Pickles / See Invisibility of the same tier, Activate's RemoveMutatorsById replaces the
/// old buff, and AddMutator just refreshes a buff whose id is already on the tower. With an id
/// that doesn't contain the item's name, none of those see the earlier buffs, so they stack.
[HarmonyPatch(typeof(GeraldoPurchaseManager), nameof(GeraldoPurchaseManager.Activate))]
static class ActivatePatch
{
    [ThreadStatic] internal static int depth;
    static void Prefix() => depth++;
    static void Finalizer() => depth--;
}

[HarmonyPatch(typeof(Mutable), nameof(Mutable.AddMutator))]
static class AddMutatorPatch
{
    static int next;

    static void Prefix(ref BehaviorMutator mutator)
    {
        if (ActivatePatch.depth <= 0 || mutator == null) return;
        try
        {
            var copy = mutator.MemberwiseClone().Cast<BehaviorMutator>();
            // Own caches, so the copies don't share which models they've mutated.
            copy.resultCache = new Il2CppSystem.Collections.Generic.Dictionary<Model, Model>();
            copy.mutated = new Il2CppSystem.Collections.Generic.HashSet<ObjectId>();
            copy.SetId("InfGer" + (++next));
            mutator = copy;
        }
        catch (Exception e)
        {
            MelonLogger.Warning($"InfiniteGeraldo: couldn't copy buff {mutator.id}: {e.Message}");
        }
    }
}
