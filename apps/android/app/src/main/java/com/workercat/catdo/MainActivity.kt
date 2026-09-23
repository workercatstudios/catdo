package com.workercat.catdo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.ViewModelProvider
import com.workercat.catdo.data.CatDoRepository
import com.workercat.catdo.sync.SyncClient
import com.workercat.catdo.ui.CatDoApp
import com.workercat.catdo.ui.CatDoTheme
import com.workercat.catdo.ui.CatDoViewModel

class MainActivity : ComponentActivity() {
    private val model by lazy {
        ViewModelProvider(this, object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : androidx.lifecycle.ViewModel> create(modelClass: Class<T>): T =
                CatDoRepository(applicationContext).let { CatDoViewModel(it, SyncClient(it)) } as T
        })[CatDoViewModel::class.java]
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { CatDoTheme { CatDoApp(model) } }
    }
}
