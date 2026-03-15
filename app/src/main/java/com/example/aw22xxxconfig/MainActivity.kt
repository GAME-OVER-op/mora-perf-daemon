package com.example.aw22xxxconfig

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.viewmodel.compose.viewModel
import com.example.aw22xxxconfig.ui.MoraApp
import com.example.aw22xxxconfig.ui.theme.MoraTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            MoraTheme {
                MoraApp(viewModel = viewModel(factory = MoraViewModel.factory(applicationContext)))
            }
        }
    }
}
