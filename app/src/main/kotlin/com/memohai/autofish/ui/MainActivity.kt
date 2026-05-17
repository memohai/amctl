package com.memohai.autofish.ui

import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.core.os.LocaleListCompat
import androidx.lifecycle.lifecycleScope
import com.memohai.autofish.R
import com.memohai.autofish.data.model.AppLanguage
import com.memohai.autofish.data.model.AppThemeMode
import com.memohai.autofish.data.repository.SettingsRepository
import com.memohai.autofish.ui.screens.HomeScreen
import com.memohai.autofish.ui.theme.AutoFishTheme
import dagger.hilt.android.AndroidEntryPoint
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.android.EntryPointAccessors
import dagger.hilt.components.SingletonComponent
import javax.inject.Inject

@AndroidEntryPoint
class MainActivity : AppCompatActivity() {
    @Inject lateinit var settingsRepository: SettingsRepository

    @EntryPoint
    @InstallIn(SingletonComponent::class)
    interface MainActivityEntryPoint {
        fun settingsRepository(): SettingsRepository
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        val entryPoint =
            EntryPointAccessors.fromApplication(
                applicationContext,
                MainActivityEntryPoint::class.java,
            )
        val initialThemeMode = runBlocking {
            entryPoint.settingsRepository().getServerConfig().appThemeMode
        }
        // Switch from launch theme to a concrete light/dark theme before first content frame.
        setTheme(
            if (initialThemeMode == AppThemeMode.DARK) {
                R.style.Theme_AutoFish_Dark
            } else {
                R.style.Theme_AutoFish_Light
            },
        )
        super.onCreate(savedInstanceState)

        enableEdgeToEdge()
        setContent {
            val appThemeModeFlow = remember(settingsRepository) {
                settingsRepository.serverConfig.map { it.appThemeMode }
            }
            val appThemeMode by appThemeModeFlow.collectAsState(initial = initialThemeMode)

            AutoFishTheme(
                darkTheme = appThemeMode == AppThemeMode.DARK,
                dynamicColor = false,
            ) {
                HomeScreen()
            }
        }

        lifecycleScope.launch {
            val config = settingsRepository.getServerConfig()
            val localeTags = when (config.appLanguage) {
                AppLanguage.SYSTEM -> ""
                AppLanguage.CHINESE -> "zh"
                AppLanguage.ENGLISH -> "en"
            }
            val currentTags = AppCompatDelegate.getApplicationLocales().toLanguageTags()
            if (currentTags != localeTags) {
                AppCompatDelegate.setApplicationLocales(LocaleListCompat.forLanguageTags(localeTags))
            }
        }
    }
}
