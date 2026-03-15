package com.example.aw22xxxconfig.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Home
import androidx.compose.material.icons.rounded.LightMode
import androidx.compose.material.icons.rounded.Settings
import androidx.compose.material.icons.rounded.SportsEsports
import androidx.compose.material.icons.rounded.Tune
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.data.model.ConnectionState
import com.example.aw22xxxconfig.data.model.MoraScreen
import com.example.aw22xxxconfig.ui.screens.*
import com.example.aw22xxxconfig.ui.theme.MoraBackground

@Composable
fun MoraApp(viewModel: MoraViewModel) {
    val navController = rememberNavController()
    val entry by navController.currentBackStackEntryAsState()
    val currentRoute = entry?.destination?.route
    val message by viewModel.message.collectAsState()
    val connection by viewModel.connection.collectAsState()

    Scaffold(
        containerColor = MoraBackground,
        bottomBar = {
            if (currentRoute != MoraScreen.GAME_DETAIL.route) {
                NavigationBar {
                    val items = listOf(
                        MoraScreen.HOME to ("Home" to Icons.Rounded.Home),
                        MoraScreen.PROFILES to ("Profiles" to Icons.Rounded.Tune),
                        MoraScreen.GAMES to ("Games" to Icons.Rounded.SportsEsports),
                        MoraScreen.LED to ("LED" to Icons.Rounded.LightMode),
                        MoraScreen.SETTINGS to ("Settings" to Icons.Rounded.Settings),
                    )
                    items.forEach { (screen, spec) ->
                        NavigationBarItem(
                            selected = currentRoute == screen.route,
                            onClick = { navController.navigate(screen.route) { launchSingleTop = true } },
                            icon = { Icon(spec.second, contentDescription = spec.first) },
                            label = { Text(spec.first) },
                        )
                    }
                }
            }
        },
        snackbarHost = {
            SnackbarHost(hostState = remember { SnackbarHostState() }) {
                if (message != null) {
                    Snackbar { Text(message ?: "") }
                }
            }
        }
    ) { padding ->
        Surface(
            modifier = Modifier
                .fillMaxSize()
                .background(MoraBackground)
                .padding(padding),
            color = MoraBackground,
        ) {
            when (connection) {
                is ConnectionState.Loading -> LoadingScreen()
                is ConnectionState.Error -> ErrorScreen(
                    message = (connection as ConnectionState.Error).message,
                    onRetry = viewModel::bootstrap,
                )
                is ConnectionState.Ready -> {
                    NavHost(navController = navController, startDestination = MoraScreen.HOME.route) {
                        composable(MoraScreen.HOME.route) { HomeScreen(viewModel) }
                        composable(MoraScreen.PROFILES.route) { ProfilesScreen(viewModel) }
                        composable(MoraScreen.GAMES.route) {
                            GamesScreen(viewModel) { pkg ->
                                navController.navigate(MoraScreen.GAME_DETAIL.detailRoute(pkg))
                            }
                        }
                        composable(MoraScreen.LED.route) { LedScreen(viewModel) }
                        composable(MoraScreen.SETTINGS.route) { SettingsScreen(viewModel) }
                        composable(
                            route = MoraScreen.GAME_DETAIL.route,
                            arguments = listOf(navArgument("packageName") { type = NavType.StringType })
                        ) { backStack ->
                            GameDetailScreen(
                                viewModel = viewModel,
                                packageName = backStack.arguments?.getString("packageName").orEmpty(),
                                onBack = { navController.popBackStack() },
                            )
                        }
                    }
                }
            }
        }
    }
}
